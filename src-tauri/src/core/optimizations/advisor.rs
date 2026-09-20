use crate::core::audit::AuditLogger;
use crate::core::policy::PolicyEngine;
use crate::core::privilege::PrivilegeBroker;
use crate::models::{AuditEvent, OptimizationRecommendation, SystemProfile};
use anyhow::{bail, Result};
use luxor_helper::action::{Governor, PrivilegedAction, SysctlKey, SysctlValue};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct OptimizationAdvisor {
    #[allow(dead_code)] // consulted once per-optimization exclusions land
    policy: PolicyEngine,
    broker: PrivilegeBroker,
}

impl OptimizationAdvisor {
    pub fn new(policy: PolicyEngine) -> Self {
        Self { policy, broker: PrivilegeBroker::new() }
    }

    pub fn recommend(&self, profile: &SystemProfile) -> Result<Vec<OptimizationRecommendation>> {
        let mut out = Vec::new();

        if profile.ram_bytes < 8 * 1024 * 1024 * 1024 {
            out.push(OptimizationRecommendation {
                id: "low-ram-zram".to_string(),
                title: "Enable zram-backed swap policy".to_string(),
                profile: "low-ram".to_string(),
                rationale: "Systems with tight memory benefit from compressed swap before disk thrash starts.".to_string(),
                command_preview: vec!["install zram generator package".to_string(), "write /etc/systemd/zram-generator.conf".to_string()],
                reversible: true,
                requires_root: true,
                risk_score: 0.24,
            });
        }

        if profile.battery_present {
            out.push(OptimizationRecommendation {
                id: "battery-profile".to_string(),
                title: "Tune power profile for battery longevity".to_string(),
                profile: "laptop".to_string(),
                rationale: "Laptop systems should expose a guided performance-versus-battery profile instead of pretending one profile fits all.".to_string(),
                command_preview: vec!["detect powerprofilesctl or tuned".to_string(), "preview profile switch".to_string()],
                reversible: true,
                requires_root: false,
                risk_score: 0.11,
            });
        }

        if profile.disks.iter().any(|d| d.is_ssd) {
            out.push(OptimizationRecommendation {
                id: "ssd-trim-check".to_string(),
                title: "Validate periodic TRIM".to_string(),
                profile: "ssd".to_string(),
                rationale: "SSDs should confirm fstrim scheduling rather than receiving cargo-cult sysctl nonsense.".to_string(),
                command_preview: vec!["systemctl status fstrim.timer".to_string()],
                reversible: false,
                requires_root: false,
                risk_score: 0.04,
            });
        }

        if profile.services.iter().any(|s| s.contains("bluetooth")) {
            out.push(OptimizationRecommendation {
                id: "startup-service-review".to_string(),
                title: "Review always-on services".to_string(),
                profile: "general".to_string(),
                rationale: "Only recommend startup service tuning as review-first, because disabling the wrong service is how people create weekend projects they never wanted.".to_string(),
                command_preview: vec!["systemctl is-enabled bluetooth.service".to_string(), "systemctl status bluetooth.service".to_string()],
                reversible: true,
                requires_root: true,
                risk_score: 0.48,
            });
        }

        if profile.cpu_model.to_lowercase().contains("intel") || profile.cpu_model.to_lowercase().contains("amd") {
            out.push(OptimizationRecommendation {
                id: "cpu-pstate-governor".to_string(),
                title: "Optimize CPU scaling governor".to_string(),
                profile: "performance".to_string(),
                rationale: "Modern Intel/AMD CPUs often default to 'powersave' or 'schedutil'. Switching to 'performance' for workstation tasks can reduce latency.".to_string(),
                command_preview: vec!["cpupower frequency-set -g performance".to_string()],
                reversible: true,
                requires_root: true,
                risk_score: 0.15,
            });
        }

        if profile.gpu_model.to_lowercase().contains("nvidia") && (profile.gpu_model.to_lowercase().contains("intel") || profile.gpu_model.to_lowercase().contains("amd")) {
             out.push(OptimizationRecommendation {
                id: "dual-gpu-hybrid".to_string(),
                title: "Configure hybrid graphics switching".to_string(),
                profile: "laptop".to_string(),
                rationale: "Dual-GPU systems often require explicit configuration for power-efficient switching.".to_string(),
                command_preview: vec!["check envycontrol or optimus-manager status".to_string()],
                reversible: true,
                requires_root: true,
                risk_score: 0.32,
            });
        }

        out.push(OptimizationRecommendation {
            id: "swappiness-tuning".to_string(),
            title: "Tune vm.swappiness for desktop usage".to_string(),
            profile: "general".to_string(),
            rationale: "Reducing swappiness (e.g. to 10) can prevent premature swap-out on systems with sufficient RAM.".to_string(),
            command_preview: vec!["sysctl -w vm.swappiness=10".to_string(), "persist in /etc/sysctl.d/99-luxor.conf".to_string()],
            reversible: true,
            requires_root: true,
            risk_score: 0.08,
        });

        Ok(out)
    }

    /// Apply an optimization by running each of its actions through the
    /// privilege broker, recording one audit event per action.
    ///
    /// The helper re-reads the system after each change, so the before/after
    /// pair in the audit log is observed state rather than intended state.
    pub fn apply(&self, id: &str, logger: &AuditLogger) -> Result<()> {
        let actions = self.actions_for(id)?;
        let risk = self.risk_for(id);

        for action in actions {
            let response = self.broker.apply(&action)?;
            logger.record_event(
                "apply-optimization",
                id,
                json!(response.before),
                json!(response.after),
                json!({
                    "action": action.kind(),
                    "effect": response.effect,
                    "persistent": action.is_persistent(),
                }),
                risk,
                0.5,
            )?;
        }
        Ok(())
    }

    /// Reverse a previously applied optimization using the state recorded at
    /// apply time, and log the reversal as its own audited event.
    pub fn rollback(&self, event: &AuditEvent, logger: &AuditLogger) -> Result<()> {
        let action = self.inverse_action(event)?;
        let response = self.broker.apply(&action)?;
        logger.record_event(
            "rollback-optimization",
            &event.target,
            json!(response.before),
            json!(response.after),
            json!({
                "action": action.kind(),
                "effect": response.effect,
                "reverses_event": event.event_id,
            }),
            self.risk_for(&event.target),
            0.5,
        )?;
        Ok(())
    }

    /// Build the action that undoes `event`, using the value captured before
    /// the original change.
    fn inverse_action(&self, event: &AuditEvent) -> Result<PrivilegedAction> {
        let recorded = event
            .before
            .as_str()
            .map(str::to_string)
            .or_else(|| event.before.as_u64().map(|n| n.to_string()));

        match event.target.as_str() {
            "swappiness-tuning" => self.restore_sysctl(SysctlKey::VmSwappiness, recorded),
            "vfs-cache-pressure" => self.restore_sysctl(SysctlKey::VmVfsCachePressure, recorded),
            "inotify-watch-limit" => {
                self.restore_sysctl(SysctlKey::FsInotifyMaxUserWatches, recorded)
            }
            "cpu-pstate-governor" => {
                let governor = match recorded.as_deref() {
                    Some("performance") => Governor::Performance,
                    Some("powersave") => Governor::Powersave,
                    Some("schedutil") => Governor::Schedutil,
                    Some("ondemand") => Governor::Ondemand,
                    Some("conservative") => Governor::Conservative,
                    other => bail!(
                        "cannot roll back to unrecognised governor {:?}; set one manually",
                        other.unwrap_or("<unrecorded>")
                    ),
                };
                Ok(PrivilegedAction::SetCpuGovernor { governor })
            }
            other => bail!("Rollback for {other} is not implemented"),
        }
    }

    fn restore_sysctl(&self, key: SysctlKey, recorded: Option<String>) -> Result<PrivilegedAction> {
        let Some(raw) = recorded else {
            bail!("audit event for {} has no recorded prior value", key.name());
        };
        // A drop-in that was absent before should go back to absent, not to a
        // guessed default.
        if raw == "<not persisted>" {
            return Ok(PrivilegedAction::ClearPersistedSysctl { key });
        }
        let parsed: u64 = raw
            .parse()
            .map_err(|_| anyhow::anyhow!("recorded value {raw:?} for {} is not a number", key.name()))?;
        Ok(PrivilegedAction::SetSysctl { key, value: SysctlValue::parse(key, parsed)? })
    }

    /// Risk weighting carried into the audit record for an optimization.
    fn risk_for(&self, id: &str) -> f32 {
        match id {
            "swappiness-tuning" => 0.08,
            "vfs-cache-pressure" => 0.10,
            "inotify-watch-limit" => 0.06,
            "cpu-pstate-governor" => 0.15,
            _ => 0.30,
        }
    }

    /// Describe what applying an optimization would do, without doing it.
    ///
    /// The description comes from the helper validating the real action against
    /// the real machine, so it cannot drift from what `apply` goes on to run.
    pub fn preview(&self, id: &str) -> Result<Vec<String>> {
        self.actions_for(id)?
            .iter()
            .map(|action| {
                self.broker
                    .preview(action)
                    .map(|response| match response.before {
                        Some(before) => format!("{} (currently {})", response.effect, before),
                        None => response.effect,
                    })
            })
            .collect()
    }

    /// The exact privileged actions an optimization performs.
    ///
    /// `apply` and `preview` both route through this, so the preview a user
    /// approves is by construction the work that runs.
    fn actions_for(&self, id: &str) -> Result<Vec<PrivilegedAction>> {
        let actions = match id {
            "swappiness-tuning" => {
                let value = SysctlValue::parse(SysctlKey::VmSwappiness, 10)?;
                vec![
                    PrivilegedAction::SetSysctl { key: SysctlKey::VmSwappiness, value },
                    PrivilegedAction::PersistSysctl { key: SysctlKey::VmSwappiness, value },
                ]
            }
            "cpu-pstate-governor" => {
                vec![PrivilegedAction::SetCpuGovernor { governor: Governor::Performance }]
            }
            "vfs-cache-pressure" => {
                let value = SysctlValue::parse(SysctlKey::VmVfsCachePressure, 50)?;
                vec![
                    PrivilegedAction::SetSysctl { key: SysctlKey::VmVfsCachePressure, value },
                    PrivilegedAction::PersistSysctl { key: SysctlKey::VmVfsCachePressure, value },
                ]
            }
            "inotify-watch-limit" => {
                let value = SysctlValue::parse(SysctlKey::FsInotifyMaxUserWatches, 524_288)?;
                vec![
                    PrivilegedAction::SetSysctl { key: SysctlKey::FsInotifyMaxUserWatches, value },
                    PrivilegedAction::PersistSysctl { key: SysctlKey::FsInotifyMaxUserWatches, value },
                ]
            }
            _ => bail!("Optimization {id} has no automated apply path"),
        };
        Ok(actions)
    }
}

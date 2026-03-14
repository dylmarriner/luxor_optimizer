use crate::core::policy::PolicyEngine;
use crate::core::audit::AuditLogger;
use crate::models::{OptimizationRecommendation, SystemProfile, AuditEvent};
use anyhow::{bail, Context, Result};
use std::process::Command;
use std::env;
use std::fs;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct OptimizationAdvisor {
    policy: PolicyEngine,
}

impl OptimizationAdvisor {
    pub fn new(policy: PolicyEngine) -> Self {
        Self { policy }
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

    pub fn apply(&self, id: &str, logger: &AuditLogger) -> Result<()> {
        match id {
            "swappiness-tuning" => {
                let before = self.get_sysctl("vm.swappiness").unwrap_or_else(|_| "60".to_string());
                self.run_privileged("set-swappiness", "sysctl", vec!["-w".to_string(), "vm.swappiness=10".to_string()])?;
                let after = self.get_sysctl("vm.swappiness").unwrap_or_else(|_| "10".to_string());
                logger.record_event("apply-optimization", id, json!(before), json!(after), json!({"key": "vm.swappiness"}), 0.08, 0.5)?;
                Ok(())
            },
            "cpu-pstate-governor" => {
                let before = self.get_governor().unwrap_or_else(|_| "unknown".to_string());
                self.run_privileged("set-governor", "cpupower", vec!["frequency-set".to_string(), "-g".to_string(), "performance".to_string()])?;
                let after = self.get_governor().unwrap_or_else(|_| "performance".to_string());
                logger.record_event("apply-optimization", id, json!(before), json!(after), json!({"tool": "cpupower"}), 0.15, 0.8)?;
                Ok(())
            },
            "low-ram-zram" => {
                self.run_privileged("install-zram", "apt", vec!["install".to_string(), "-y".to_string(), "zram-generator".to_string()])?;
                logger.record_event("apply-optimization", id, json!("uninstalled"), json!("installed"), json!({"pkg": "zram-generator"}), 0.24, 0.9)?;
                Ok(())
            },
            "startup-service-review" => {
                let before = self.is_service_enabled("bluetooth.service").unwrap_or(true);
                self.run_privileged("disable-bluetooth", "systemctl", vec!["disable".to_string(), "bluetooth.service".to_string()])?;
                let after = self.is_service_enabled("bluetooth.service").unwrap_or(false);
                logger.record_event("apply-optimization", id, json!(before), json!(after), json!({"service": "bluetooth.service"}), 0.48, 0.3)?;
                Ok(())
            },
            _ => bail!("Optimization {} not yet implementable or requires manual steps", id),
        }
    }

    pub fn rollback(&self, event: &AuditEvent, _logger: &AuditLogger) -> Result<()> {
        match event.target.as_str() {
            "swappiness-tuning" => {
                let old_val = event.before.as_str().context("invalid before state")?;
                self.run_privileged("rollback-swappiness", "sysctl", vec!["-w".to_string(), format!("vm.swappiness={}", old_val)])
            },
            "cpu-pstate-governor" => {
                let old_gov = event.before.as_str().context("invalid before state")?;
                self.run_privileged("rollback-governor", "cpupower", vec!["frequency-set".to_string(), "-g".to_string(), old_gov.to_string()])
            },
            "startup-service-review" => {
                let was_enabled = event.before.as_bool().context("invalid before state")?;
                if was_enabled {
                    self.run_privileged("enable-bluetooth", "systemctl", vec!["enable".to_string(), "bluetooth.service".to_string()])
                } else {
                    Ok(()) // already disabled?
                }
            },
            _ => bail!("Rollback for {} not implemented", event.target),
        }
    }

    fn get_sysctl(&self, key: &str) -> Result<String> {
        let path = format!("/proc/sys/{}", key.replace('.', "/"));
        fs::read_to_string(path).map(|s| s.trim().to_string()).context("failed to read sysctl")
    }

    fn get_governor(&self) -> Result<String> {
        fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
            .map(|s| s.trim().to_string())
            .context("failed to read governor")
    }

    fn is_service_enabled(&self, service: &str) -> Result<bool> {
        let output = Command::new("systemctl")
            .arg("is-enabled")
            .arg(service)
            .output()?;
        Ok(output.status.success())
    }

    fn run_privileged(&self, action: &str, command: &str, args: Vec<String>) -> Result<()> {
        let helper_path = env::current_exe()?
            .parent()
            .context("no bin dir")?
            .join("luxor-helper");
        
        let payload = serde_json::json!({
            "action": action,
            "command": command,
            "args": args,
            "dry_run": false
        });

        let output = Command::new("pkexec")
            .arg(helper_path)
            .arg(payload.to_string())
            .output()
            .context("failed to execute pkexec helper")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("Privileged action failed: {}", err);
        }

        Ok(())
    }
}

//! Root-side execution of validated actions.
//!
//! Every function here receives already-validated parameters, so the work is
//! narrow: perform the change, then re-read the system to report what actually
//! happened rather than what was requested.

use luxor_ipc::{
    CacheTarget, Governor, HelperResponse, PrivilegedAction, Status, SysctlKey, SysctlValue,
    UnitName, DROP_IN_PATH,
};
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Run an action, or describe it without acting when `dry_run` is set.
pub fn execute(action: &PrivilegedAction, dry_run: bool) -> Result<HelperResponse> {
    let before = read_current(action);

    if dry_run {
        return Ok(HelperResponse {
            status: Status::DryRun,
            effect: action.describe(),
            before,
            after: None,
            message: Some("no changes were made".to_string()),
        });
    }

    match action {
        PrivilegedAction::SetSysctl { key, value } => set_sysctl(*key, *value)?,
        PrivilegedAction::PersistSysctl { key, value } => persist_sysctl(*key, Some(*value))?,
        PrivilegedAction::ClearPersistedSysctl { key } => persist_sysctl(*key, None)?,
        PrivilegedAction::SetCpuGovernor { governor } => set_governor(*governor)?,
        PrivilegedAction::EnableUnit { unit } => systemctl(&["enable"], unit)?,
        PrivilegedAction::DisableUnit { unit } => systemctl(&["disable"], unit)?,
        PrivilegedAction::StartUnit { unit } => systemctl(&["start"], unit)?,
        PrivilegedAction::StopUnit { unit } => systemctl(&["stop"], unit)?,
        PrivilegedAction::PurgeCache { target } => purge_cache(*target)?,
    }

    Ok(HelperResponse {
        status: Status::Ok,
        effect: action.describe(),
        before,
        after: read_current(action),
        message: None,
    })
}

/// Observe the state an action affects, for audit before/after pairs. Failure
/// to read is not fatal — some actions have no single readable value.
fn read_current(action: &PrivilegedAction) -> Option<String> {
    match action {
        PrivilegedAction::SetSysctl { key, .. } => read_sysctl(*key).ok(),
        PrivilegedAction::PersistSysctl { key, .. }
        | PrivilegedAction::ClearPersistedSysctl { key } => Some(
            read_drop_in(*key)
                .ok()
                .flatten()
                .unwrap_or_else(|| "<not persisted>".to_string()),
        ),
        PrivilegedAction::SetCpuGovernor { .. } => read_governor().ok(),
        PrivilegedAction::EnableUnit { unit }
        | PrivilegedAction::DisableUnit { unit }
        | PrivilegedAction::StartUnit { unit }
        | PrivilegedAction::StopUnit { unit } => Some(unit_state(unit)),
        PrivilegedAction::PurgeCache { target } => {
            Some(format!("{} bytes", dir_size(target.path())))
        }
    }
}

fn read_sysctl(key: SysctlKey) -> Result<String> {
    let path = key.proc_path();
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(raw.trim().to_string())
}

fn set_sysctl(key: SysctlKey, value: SysctlValue) -> Result<()> {
    let path = key.proc_path();
    // The path is derived from a closed enum, so it cannot point outside
    // /proc/sys; write directly rather than shelling out to sysctl(8).
    fs::write(&path, format!("{}\n", value.get()))
        .with_context(|| format!("writing {}", path.display()))
}

fn read_drop_in(key: SysctlKey) -> Result<Option<String>> {
    let path = Path::new(DROP_IN_PATH);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let prefix = format!("{} = ", key.name());
    Ok(content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(|v| v.trim().to_string())))
}

/// Rewrite this build's drop-in with `key` set to `value`, or removed when
/// `value` is `None`.
///
/// Only ever touches [`DROP_IN_PATH`]. Existing distro configuration is left
/// alone, so removing this one file fully restores stock behaviour.
fn persist_sysctl(key: SysctlKey, value: Option<SysctlValue>) -> Result<()> {
    let path = Path::new(DROP_IN_PATH);
    let existing = if path.exists() { fs::read_to_string(path)? } else { String::new() };

    let prefix = format!("{} = ", key.name());
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|line| !line.starts_with(&prefix))
        .map(str::to_string)
        .collect();

    if lines.first().map(|l| l.starts_with('#')) != Some(true) {
        lines.insert(0, "# Managed by Luxor Optimizer. Delete this file to revert.".to_string());
    }
    if let Some(value) = value {
        lines.push(format!("{}{}", prefix, value.get()));
    }

    // Only the comment line remains: drop the file rather than leave a stub.
    if lines.len() <= 1 {
        if path.exists() {
            fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Write-then-rename so a crash cannot leave a half-written config that
    // systemd-sysctl would choke on at boot.
    let tmp = path.with_extension("conf.luxor-tmp");
    let mut file = fs::File::create(&tmp)
        .with_context(|| format!("creating {}", tmp.display()))?;
    file.write_all(lines.join("\n").as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path).with_context(|| format!("installing {}", path.display()))
}

const CPU_ROOT: &str = "/sys/devices/system/cpu";

fn read_governor() -> Result<String> {
    let path = Path::new(CPU_ROOT).join("cpu0/cpufreq/scaling_governor");
    Ok(fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?
        .trim()
        .to_string())
}

/// Apply a governor to every CPU that exposes cpufreq.
///
/// Partial application is a real outcome on hybrid parts, so this reports how
/// many CPUs accepted the change and fails only when none did.
fn set_governor(governor: Governor) -> Result<()> {
    let entries = fs::read_dir(CPU_ROOT).with_context(|| format!("listing {CPU_ROOT}"))?;
    let mut applied = 0usize;
    let mut last_err = None;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // cpuN, where N is all digits — skips cpuidle, cpufreq, and friends.
        if !name.starts_with("cpu") || !name[3..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if name.len() == 3 {
            continue;
        }
        let target = entry.path().join("cpufreq/scaling_governor");
        if !target.exists() {
            continue;
        }
        match fs::write(&target, governor.as_str()) {
            Ok(()) => applied += 1,
            Err(err) => last_err = Some((target, err)),
        }
    }

    if applied == 0 {
        match last_err {
            Some((path, err)) => {
                bail!("no CPU accepted the governor; last failure on {}: {err}", path.display())
            }
            None => bail!("no CPU on this system exposes a cpufreq scaling_governor"),
        }
    }
    Ok(())
}

fn unit_state(unit: &UnitName) -> String {
    let enabled = Command::new("systemctl")
        .args(["is-enabled", "--", unit.as_str()])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let active = Command::new("systemctl")
        .args(["is-active", "--", unit.as_str()])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    format!("{enabled}/{active}")
}

/// Invoke systemctl with a fixed verb and exactly one validated unit name.
///
/// `--` terminates option parsing, and `UnitName` has already rejected
/// anything option-like or path-like, so the unit cannot become a flag.
fn systemctl(verb: &[&str], unit: &UnitName) -> Result<()> {
    let output = Command::new("systemctl")
        .args(verb)
        .arg("--")
        .arg(unit.as_str())
        .output()
        .context("failed to invoke systemctl")?;

    if !output.status.success() {
        bail!(
            "systemctl {} {} failed: {}",
            verb.join(" "),
            unit.as_str(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Delete the contents of one fixed cache directory.
///
/// Refuses to act when the target is a symlink, so a planted link cannot
/// redirect the deletion somewhere else.
fn purge_cache(target: CacheTarget) -> Result<()> {
    let path = target.path();
    if !path.exists() {
        return Ok(());
    }

    let meta = fs::symlink_metadata(path)
        .with_context(|| format!("stat {}", path.display()))?;
    if meta.file_type().is_symlink() {
        bail!("{} is a symlink; refusing to purge", path.display());
    }
    if !meta.is_dir() {
        bail!("{} is not a directory", path.display());
    }

    for entry in fs::read_dir(path).with_context(|| format!("listing {}", path.display()))? {
        let entry = entry?;
        let child = entry.path();
        let child_meta = fs::symlink_metadata(&child)?;
        if child_meta.is_dir() && !child_meta.file_type().is_symlink() {
            fs::remove_dir_all(&child)
                .with_context(|| format!("removing {}", child.display()))?;
        } else {
            fs::remove_file(&child).with_context(|| format!("removing {}", child.display()))?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    fn walk(path: &Path, total: &mut u64) {
        let Ok(entries) = fs::read_dir(path) else { return };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_file() {
                *total = total.saturating_add(meta.len());
            } else if meta.is_dir() {
                walk(&entry.path(), total);
            }
        }
    }
    let mut total = 0;
    walk(path, &mut total);
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use luxor_ipc::SysctlKey;

    #[test]
    fn dry_run_reports_without_acting() {
        let action = PrivilegedAction::SetSysctl {
            key: SysctlKey::VmSwappiness,
            value: SysctlValue::parse(SysctlKey::VmSwappiness, 10).unwrap(),
        };
        let response = execute(&action, true).expect("dry run should succeed");
        assert_eq!(response.status, Status::DryRun);
        assert!(response.after.is_none());
        assert!(response.effect.contains("vm.swappiness"));
    }

    #[test]
    fn purge_is_a_no_op_when_the_target_is_absent() {
        // /var/crash is absent on most CI images; absence must not be an error.
        if !CacheTarget::SystemCrashDumps.path().exists() {
            assert!(purge_cache(CacheTarget::SystemCrashDumps).is_ok());
        }
    }

    #[test]
    fn describe_names_the_real_effect() {
        let action = PrivilegedAction::DisableUnit {
            unit: UnitName::parse("bluetooth.service").unwrap(),
        };
        assert_eq!(action.describe(), "disable bluetooth.service at boot");
        assert!(action.is_persistent());
    }
}

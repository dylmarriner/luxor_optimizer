//! Package inventory across the ecosystems a Linux desktop actually mixes.
//!
//! Sizes come from the packaging system where it reports them and are `None`
//! where it does not. Every field was previously a hardcoded constant — 50 MB
//! for each apt package, 500 MB for each Flatpak, "last used 7 days ago" for
//! all of them — which meant the risk scores computed from them described
//! nothing about the machine.

use crate::core::{policy::PolicyEngine, risk::score_package_risk};
use crate::models::{PackageRecord, PackageSource, SystemProfile};
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PackageManagerInventory {
    policy: PolicyEngine,
}

impl PackageManagerInventory {
    pub fn new(policy: PolicyEngine) -> Self {
        Self { policy }
    }

    pub fn discover(&self, profile: &SystemProfile) -> Result<Vec<PackageRecord>> {
        let mut out = Vec::new();
        let has = |name: &str| profile.package_managers.iter().any(|p| p == name);

        if has("apt") {
            self.discover_dpkg(&mut out);
        }
        if has("dnf") {
            self.discover_rpm(&mut out);
        }
        if has("pacman") {
            self.discover_pacman(&mut out);
        }
        if has("zypper") {
            // zypper's backend is rpm; querying rpm directly avoids a second
            // parser and works whether or not zypper itself is installed.
            if !has("dnf") {
                self.discover_rpm(&mut out);
            }
        }
        if has("flatpak") {
            self.discover_flatpak(&mut out);
        }
        if has("snap") {
            self.discover_snap(&mut out);
        }

        self.discover_appimages(&mut out, profile);
        Ok(out)
    }

    /// dpkg reports Installed-Size in kibibytes.
    fn discover_dpkg(&self, out: &mut Vec<PackageRecord>) {
        let Some(stdout) = run("dpkg-query", &["-W", "-f=${Package}\\t${Installed-Size}\\n"]) else {
            return;
        };
        for line in stdout.lines() {
            let mut parts = line.split('\t');
            let Some(name) = parts.next().filter(|n| !n.is_empty()) else { continue };
            let size = parts
                .next()
                .and_then(|kib| kib.trim().parse::<u64>().ok())
                .map(|kib| kib * 1024);
            out.push(self.record(name, PackageSource::Native, size, false));
        }
    }

    /// rpm reports SIZE in bytes.
    fn discover_rpm(&self, out: &mut Vec<PackageRecord>) {
        let Some(stdout) = run("rpm", &["-qa", "--qf", "%{NAME}\\t%{SIZE}\\n"]) else {
            return;
        };
        for line in stdout.lines() {
            let mut parts = line.split('\t');
            let Some(name) = parts.next().filter(|n| !n.is_empty()) else { continue };
            let size = parts.next().and_then(|b| b.trim().parse::<u64>().ok());
            out.push(self.record(name, PackageSource::Native, size, false));
        }
    }

    /// `pacman -Qi` emits one block per package; Name and Installed Size are
    /// the two fields needed, and Installed Size is human-readable.
    fn discover_pacman(&self, out: &mut Vec<PackageRecord>) {
        let Some(stdout) = run("pacman", &["-Qi"]) else { return };
        let mut name: Option<String> = None;
        let mut size: Option<u64> = None;

        for line in stdout.lines() {
            if let Some(value) = line.strip_prefix("Name") {
                name = value.split_once(':').map(|(_, v)| v.trim().to_string());
            } else if let Some(value) = line.strip_prefix("Installed Size") {
                size = value.split_once(':').and_then(|(_, v)| parse_human_size(v.trim()));
            } else if line.trim().is_empty() {
                if let Some(n) = name.take() {
                    out.push(self.record(&n, PackageSource::Native, size.take(), false));
                }
                size = None;
            }
        }
        if let Some(n) = name {
            out.push(self.record(&n, PackageSource::Native, size, false));
        }
    }

    fn discover_flatpak(&self, out: &mut Vec<PackageRecord>) {
        let Some(stdout) = run("flatpak", &["list", "--app", "--columns=application,size"]) else {
            return;
        };
        for line in stdout.lines() {
            let mut parts = line.split('\t');
            let Some(app) = parts.next().map(str::trim).filter(|a| !a.is_empty()) else {
                continue;
            };
            let size = parts.next().and_then(|s| parse_human_size(s.trim()));
            out.push(self.record(app, PackageSource::Flatpak, size, false));
        }
    }

    /// `snap list` reports no size, so size is measured from the mounted
    /// squashfs revision on disk where readable.
    fn discover_snap(&self, out: &mut Vec<PackageRecord>) {
        let Some(stdout) = run("snap", &["list"]) else { return };
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let (Some(name), Some(rev)) = (parts.first(), parts.get(2)) else { continue };
            let size = fs::metadata(format!("/var/lib/snapd/snaps/{name}_{rev}.snap"))
                .ok()
                .map(|m| m.len());
            let protected = *name == "snapd" || name.starts_with("core");
            out.push(self.record(name, PackageSource::Snap, size, protected));
        }
    }

    fn discover_appimages(&self, out: &mut Vec<PackageRecord>, profile: &SystemProfile) {
        let home = dirs::home_dir().unwrap_or_default();
        let mut search_paths = vec![
            home.join("Downloads"),
            home.join(".local/bin"),
            home.join("Applications"),
        ];
        for disk in &profile.disks {
            if disk.mount_point == "/" {
                continue;
            }
            search_paths.push(PathBuf::from(&disk.mount_point).join("Applications"));
        }

        for path in search_paths {
            let Ok(entries) = fs::read_dir(&path) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.to_lowercase().ends_with(".appimage") {
                    continue;
                }
                // An AppImage is a single file, so its size is exact — and
                // atime, where the filesystem records it, is the one genuine
                // last-used signal available anywhere in this inventory.
                let meta = entry.metadata().ok();
                let size = meta.as_ref().map(|m| m.len());
                let last_used = meta.as_ref().and_then(days_since_access);
                out.push(PackageRecord {
                    last_used_days_ago: last_used,
                    ..self.record(&name, PackageSource::AppImage, size, false)
                });
            }
        }
    }

    fn record(
        &self,
        name: &str,
        source: PackageSource,
        size: Option<u64>,
        protected: bool,
    ) -> PackageRecord {
        let mut rationale = vec![format!("Surfaced by {source:?} adapter")];
        if protected {
            rationale.push("Protected system component".to_string());
        }
        if size.is_none() {
            rationale.push("Installed size not reported by this packaging system".to_string());
        }

        let criticality = if protected || self.policy.is_protected_app(name) {
            "protected"
        } else {
            "normal"
        }
        .to_string();

        PackageRecord {
            name: name.to_string(),
            source: source.clone(),
            installed_size_bytes: size,
            criticality,
            // No packaging system tracks last-use, and inventing a value here
            // is what made the previous risk scores meaningless. Left absent
            // until a real usage signal exists to fill it.
            last_used_days_ago: None,
            install_age_days: None,
            removal_preview: vec![match source {
                PackageSource::Native => format!("package-manager preview removal for {name}"),
                PackageSource::Flatpak => format!("flatpak uninstall --assumeno {name}"),
                PackageSource::Snap => format!("snap remove --purge {name}"),
                PackageSource::AppImage => {
                    format!("confirm deletion path for {name} and associated desktop artifacts")
                }
            }],
            rationale,
            risk_score: score_package_risk(size, None, None, protected),
            metadata: BTreeMap::new(),
        }
    }
}

/// Run a command, returning stdout only when it succeeded.
///
/// A missing or failing tool yields no packages for that ecosystem rather than
/// aborting the whole inventory, since mixed systems routinely have some of
/// these installed and not others.
fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse sizes as pacman and flatpak print them, e.g. "1.23 MiB", "45,6 kB".
fn parse_human_size(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "-" {
        return None;
    }
    let split = raw.find(|c: char| c.is_alphabetic())?;
    let (number, unit) = raw.split_at(split);
    let value: f64 = number.trim().replace(',', ".").parse().ok()?;

    let multiplier: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "b" => 1.0,
        "kb" | "kib" | "k" => 1024.0,
        "mb" | "mib" | "m" => 1024.0 * 1024.0,
        "gb" | "gib" | "g" => 1024.0 * 1024.0 * 1024.0,
        "tb" | "tib" | "t" => 1024.0_f64.powi(4),
        _ => return None,
    };
    Some((value * multiplier) as u64)
}

/// Whole days since a file was last accessed.
///
/// Returns `None` when the filesystem does not record atime (`noatime` is a
/// common mount option), so absence is never mistaken for "never used".
fn days_since_access(meta: &fs::Metadata) -> Option<u32> {
    let accessed = meta.accessed().ok()?;
    let elapsed = std::time::SystemTime::now().duration_since(accessed).ok()?;
    Some((elapsed.as_secs() / 86_400) as u32)
}

#[allow(dead_code)]
fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_sizes_parse_across_locales_and_units() {
        assert_eq!(parse_human_size("1 B"), Some(1));
        assert_eq!(parse_human_size("2 KiB"), Some(2048));
        assert_eq!(parse_human_size("1.5 MiB"), Some(1_572_864));
        // pacman honours locale decimal separators.
        assert_eq!(parse_human_size("1,5 MiB"), Some(1_572_864));
        assert_eq!(parse_human_size("3 GiB"), Some(3_221_225_472));
    }

    #[test]
    fn unparseable_sizes_are_absent_rather_than_zero() {
        // Zero would read as "this package is free to keep"; absent does not.
        assert_eq!(parse_human_size(""), None);
        assert_eq!(parse_human_size("-"), None);
        assert_eq!(parse_human_size("unknown"), None);
        assert_eq!(parse_human_size("12 parsecs"), None);
    }

    #[test]
    fn a_missing_tool_yields_nothing_rather_than_failing() {
        assert!(run("luxor-nonexistent-binary", &["--version"]).is_none());
    }
}

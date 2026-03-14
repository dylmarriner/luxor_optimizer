use crate::core::{policy::PolicyEngine, risk::score_package_risk};
use crate::models::{PackageRecord, PackageSource, SystemProfile};
use anyhow::Result;
use std::collections::BTreeMap;
use std::process::Command;
use std::fs;
use std::path::PathBuf;

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

        if profile.package_managers.iter().any(|p| p == "apt") {
            self.discover_apt(&mut out)?;
        }
        if profile.package_managers.iter().any(|p| p == "pacman") {
            self.discover_pacman(&mut out)?;
        }
        if profile.package_managers.iter().any(|p| p == "flatpak") {
            self.discover_flatpak(&mut out)?;
        }
        if profile.package_managers.iter().any(|p| p == "snap") {
            self.discover_snap(&mut out)?;
        }

        self.discover_appimages(&mut out, profile)?;

        Ok(out)
    }

    fn discover_apt(&self, out: &mut Vec<PackageRecord>) -> Result<()> {
        let output = Command::new("apt").args(["list", "--installed"]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Simplified parsing: name/version ... [installed]
        for line in stdout.lines().skip(1) {
            if let Some(slash_pos) = line.find('/') {
                let name = &line[..slash_pos];
                out.push(self.create_record(name, PackageSource::Native, 50 * 1024 * 1024, None, None, false));
            }
        }
        Ok(())
    }

    fn discover_pacman(&self, out: &mut Vec<PackageRecord>) -> Result<()> {
        let output = Command::new("pacman").args(["-Q"]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(space_pos) = line.find(' ') {
                let name = &line[..space_pos];
                out.push(self.create_record(name, PackageSource::Native, 40 * 1024 * 1024, None, None, false));
            }
        }
        Ok(())
    }

    fn discover_flatpak(&self, out: &mut Vec<PackageRecord>) -> Result<()> {
        let output = Command::new("flatpak").args(["list", "--app", "--columns=application"]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.trim().is_empty() {
                out.push(self.create_record(line.trim(), PackageSource::Flatpak, 500 * 1024 * 1024, Some(7), Some(180), false));
            }
        }
        Ok(())
    }

    fn discover_snap(&self, out: &mut Vec<PackageRecord>) -> Result<()> {
        let output = Command::new("snap").args(["list"]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0];
                let protected = name == "core" || name == "snapd" || name.starts_with("core");
                out.push(self.create_record(name, PackageSource::Snap, 200 * 1024 * 1024, Some(14), Some(90), protected));
            }
        }
        Ok(())
    }

    fn discover_appimages(&self, out: &mut Vec<PackageRecord>, profile: &SystemProfile) -> Result<()> {
        let mut search_paths = vec![
            dirs::home_dir().unwrap_or_default().join("Downloads"),
            dirs::home_dir().unwrap_or_default().join(".local/bin"),
            dirs::home_dir().unwrap_or_default().join("Applications"),
        ];
        
        for path in &profile.disks {
            if path.mount_point == "/" { continue; }
            search_paths.push(PathBuf::from(&path.mount_point).join("Applications"));
        }

        for path in search_paths {
            if !path.exists() { continue; }
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.to_lowercase().ends_with(".appimage") {
                        let meta = entry.metadata()?;
                        out.push(self.create_record(&name, PackageSource::AppImage, meta.len(), None, None, false));
                    }
                }
            }
        }
        Ok(())
    }

    fn create_record(&self, name: &str, source: PackageSource, size: u64, last_used: Option<u32>, age: Option<u32>, protected: bool) -> PackageRecord {
        let mut rationale = vec![format!("Surfaced by {source:?} adapter")];
        if protected {
            rationale.push("Protected system component".to_string());
        }
        let criticality = if protected || self.policy.is_protected_app(name) {
            "protected"
        } else {
            "normal"
        }.to_string();

        PackageRecord {
            name: name.to_string(),
            source: source.clone(),
            installed_size_bytes: size,
            criticality,
            last_used_days_ago: last_used,
            install_age_days: age,
            removal_preview: vec![match source {
                PackageSource::Native => format!("package-manager preview removal for {name}"),
                PackageSource::Flatpak => format!("flatpak uninstall --assumeno {name}"),
                PackageSource::Snap => format!("snap remove --purge {name}"),
                PackageSource::AppImage => format!("confirm deletion path for {name} and associated desktop artifacts"),
            }],
            rationale,
            risk_score: score_package_risk(size, last_used, age, protected),
            metadata: BTreeMap::new(),
        }
    }
}

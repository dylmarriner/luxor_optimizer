use crate::models::{DiskProfile, SystemProfile};
use anyhow::Result;
use std::{collections::HashSet, fs, path::Path, process::Command};
use sysinfo::{Disks, System};
use which::which;

#[derive(Debug, Default)]
pub struct SystemDetector;

impl SystemDetector {
    pub fn detect(&self) -> Result<SystemProfile> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
        let distro = extract_value(&os_release, "ID").unwrap_or_else(|| "linux".to_string());
        let version = extract_value(&os_release, "VERSION_ID").unwrap_or_else(|| "unknown".to_string());

        let desktop_environment = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let display_server = std::env::var("XDG_SESSION_TYPE").ok();
        let init_system = if Path::new("/run/systemd/system").exists() {
            "systemd".to_string()
        } else {
            "unknown".to_string()
        };

        let cpu_model = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let cores = sys.physical_core_count().unwrap_or_else(|| sys.cpus().len());
        let threads = sys.cpus().len();
        let ram_bytes = sys.total_memory();
        let swap_bytes = sys.total_swap();
        let kernel = System::kernel_version().unwrap_or_else(|| "unknown".to_string());
        let battery_present = Path::new("/sys/class/power_supply/BAT0").exists();
        let uptime_seconds = System::uptime();
        let gpu_model = detect_gpu();
        let power_profile = detect_power_profile();
        let disks = detect_disks();
        let package_managers = detect_package_managers();
        let services = read_enabled_services();

        Ok(SystemProfile {
            distro,
            version,
            desktop_environment,
            display_server,
            init_system,
            kernel,
            cpu_model,
            gpu_model,
            cores,
            threads,
            ram_bytes,
            swap_bytes,
            package_managers,
            battery_present,
            power_profile,
            uptime_seconds,
            disks,
            services,
        })
    }
}

fn extract_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find(|line| line.starts_with(&format!("{key}=")))
        .and_then(|line| line.split_once('=').map(|(_, value)| value.trim_matches('"').to_string()))
}

fn detect_package_managers() -> Vec<String> {
    let candidates = ["apt", "dnf", "pacman", "zypper", "flatpak", "snap"];
    candidates
        .iter()
        .filter(|cmd| which(cmd).is_ok())
        .map(|cmd| (*cmd).to_string())
        .collect()
}

fn detect_disks() -> Vec<DiskProfile> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|disk| DiskProfile {
            mount_point: disk.mount_point().display().to_string(),
            fs_type: disk.file_system().to_string_lossy().to_string(),
            kind: format!("{:?}", disk.kind()),
            is_ssd: disk.is_removable().not(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        })
        .collect()
}

fn read_enabled_services() -> Vec<String> {
    let path = Path::new("/etc/systemd/system");
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut services = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".service") && seen.insert(name.clone()) {
            services.push(name);
        }
    }
    services.sort();
    services.truncate(40);
    services
}

fn detect_gpu() -> String {
    if let Ok(output) = Command::new("lspci").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("VGA") || line.contains("3D") {
                if let Some(pos) = line.find(": ") {
                    return line[pos + 2..].to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn detect_power_profile() -> Option<String> {
    if let Ok(output) = Command::new("powerprofilesctl").arg("get").output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    None
}

trait BoolNot {
    fn not(self) -> bool;
}

impl BoolNot for bool {
    fn not(self) -> bool {
        !self
    }
}

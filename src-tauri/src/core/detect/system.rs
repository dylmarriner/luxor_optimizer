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
            is_ssd: is_solid_state(&disk.name().to_string_lossy()),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        })
        .collect()
}

/// Whether a block device is solid state, read from the kernel.
///
/// This previously returned `!is_removable()`, which made every internal
/// spinning disk report as an SSD and fired the TRIM recommendation at
/// hardware that cannot use it. The kernel already knows: `queue/rotational`
/// is 0 for SSD and NVMe, 1 for spinning media.
fn is_solid_state(device_path: &str) -> bool {
    let Some(name) = device_path.rsplit('/').next() else {
        return false;
    };

    // Walk from the partition up to its parent disk: /sys/block holds whole
    // devices, so "nvme0n1p2" must be tried as "nvme0n1" and "sda1" as "sda".
    for candidate in [name.to_string(), strip_partition(name)] {
        let path = format!("/sys/block/{candidate}/queue/rotational");
        if let Ok(raw) = fs::read_to_string(&path) {
            return raw.trim() == "0";
        }
    }
    // Device mapper, LUKS, btrfs subvolumes and the like have no single
    // backing queue. Unknown is reported as not-SSD so that SSD-only advice
    // is withheld rather than guessed at.
    false
}

fn strip_partition(name: &str) -> String {
    // nvme0n1p3 / mmcblk0p1 -> strip the trailing pN.
    if let Some(idx) = name.rfind('p') {
        let suffix = &name[idx + 1..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            let base = &name[..idx];
            if base.chars().last().is_some_and(|c| c.is_ascii_digit()) {
                return base.to_string();
            }
        }
    }
    // These families end in a digit when they are already whole devices
    // (nvme0n1, mmcblk0), so trimming trailing digits would corrupt them.
    // Having ruled out a pN suffix above, the name is the whole device.
    if name.starts_with("nvme") || name.starts_with("mmcblk") {
        return name.to_string();
    }
    // sda1 / vdb2 -> strip trailing digits
    name.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

/// Units actually enabled at boot.
///
/// This used to list directory entries under /etc/systemd/system, which is
/// neither the set of enabled units (those are symlinks inside `.wants`
/// directories) nor complete (distro units live in /usr/lib/systemd/system).
/// It also truncated at 40, so recommendations keyed off this list silently
/// missed units on most systems. Ask systemd instead.
fn read_enabled_services() -> Vec<String> {
    let Ok(output) = Command::new("systemctl")
        .args(["list-unit-files", "--type=service", "--state=enabled", "--no-legend", "--no-pager"])
        .output()
    else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut services: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| name.ends_with(".service"))
        .filter(|name| seen.insert(name.to_string()))
        .map(str::to_string)
        .collect();
    services.sort();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_names_resolve_to_their_parent_disk() {
        assert_eq!(strip_partition("nvme0n1p3"), "nvme0n1");
        assert_eq!(strip_partition("mmcblk0p1"), "mmcblk0");
        assert_eq!(strip_partition("sda1"), "sda");
        assert_eq!(strip_partition("vdb12"), "vdb");
        // Whole devices are already their own parent.
        assert_eq!(strip_partition("sda"), "sda");
        assert_eq!(strip_partition("nvme0n1"), "nvme0n1");
    }

    #[test]
    fn unknown_devices_are_reported_as_not_ssd_rather_than_guessed() {
        assert!(!is_solid_state("/dev/mapper/nonexistent-luxor-test"));
        assert!(!is_solid_state(""));
    }

    #[test]
    fn ssd_detection_agrees_with_the_kernel_for_a_real_device() {
        // Compare against /sys directly for whatever this machine actually has.
        let Ok(entries) = fs::read_dir("/sys/block") else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let rotational = format!("/sys/block/{name}/queue/rotational");
            let Ok(raw) = fs::read_to_string(&rotational) else { continue };
            let expected = raw.trim() == "0";
            assert_eq!(
                is_solid_state(&format!("/dev/{name}")),
                expected,
                "disagreed with the kernel for {name}"
            );
        }
    }
}

//! Linux capability probes.
//!
//! Each probe asks the kernel rather than assuming. The answers differ
//! meaningfully across hardware: a VM has no cpufreq, an Intel laptop on recent
//! microcode has MSR undervolting fused off, an AMD GPU needs a boot parameter
//! before its clocks are writable at all.

use super::{readable, Capability, Platform, Support};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct LinuxPlatform;

const CPU_ROOT: &str = "/sys/devices/system/cpu";

impl Platform for LinuxPlatform {
    fn id(&self) -> &'static str {
        "linux"
    }

    fn probe(&self) -> BTreeMap<Capability, Support> {
        let mut out = BTreeMap::new();
        out.insert(Capability::CpuGovernor, probe_governor());
        out.insert(Capability::CpuOverclock, probe_cpu_overclock());
        out.insert(Capability::GpuOverclock, probe_gpu_overclock());
        out.insert(Capability::SwapTuning, probe_swap());
        out.insert(Capability::KernelTunables, probe_sysctl());
        out.insert(Capability::CacheCleanup, Support::Available);
        out.insert(Capability::ProcessConstraint, probe_cgroups());
        out.insert(Capability::ServiceManagement, probe_systemd());
        out
    }
}

fn probe_governor() -> Support {
    if readable(format!("{CPU_ROOT}/cpu0/cpufreq/scaling_governor")) {
        Support::Available
    } else {
        // Common in VMs and on some ARM boards: the CPU has no scaling driver.
        Support::unsupported("no cpufreq scaling driver is present")
    }
}

fn probe_cpu_overclock() -> Support {
    // Undervolting and OC both go through model-specific registers, which need
    // the msr module loaded and root. Without /dev/cpu/0/msr there is nothing
    // to write to regardless of what the silicon would allow.
    if !readable("/dev/cpu/0/msr") {
        return Support::blocked("the msr kernel module is not loaded (try: modprobe msr)");
    }

    let vendor = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    if vendor.contains("AuthenticAMD") {
        return Support::todo("AMD support needs the ryzen_smu interface, which is not wired up");
    }
    if vendor.contains("GenuineIntel") {
        return Support::todo(
            "Intel undervolting via MSR 0x150 is not wired up, and is fused off by microcode on \
             most parts released after 2019",
        );
    }
    Support::unsupported("unrecognised CPU vendor")
}

fn probe_gpu_overclock() -> Support {
    // AMD exposes clocks through pp_od_clk_voltage, but only when the driver
    // was started with the full ppfeaturemask; otherwise the file is absent
    // even though the hardware supports it.
    for card in 0..4 {
        let device = format!("/sys/class/drm/card{card}/device");
        if !Path::new(&device).exists() {
            continue;
        }
        if readable(format!("{device}/pp_od_clk_voltage")) {
            return Support::todo("AMD pp_od_clk_voltage is present but no apply path exists yet");
        }
        if readable(format!("{device}/power_dpm_force_performance_level")) {
            return Support::blocked(
                "AMD overclocking needs amdgpu.ppfeaturemask=0xffffffff on the kernel command line",
            );
        }
    }

    if readable("/proc/driver/nvidia/version") {
        return Support::todo("NVIDIA control via NVML is not wired up");
    }
    Support::unsupported("no GPU with a writable clock interface was found")
}

fn probe_swap() -> Support {
    if !readable("/proc/swaps") {
        return Support::unsupported("this kernel reports no swap subsystem");
    }
    if readable("/sys/class/zram-control") || readable("/sys/block/zram0") {
        return Support::Available;
    }
    Support::blocked("zram is unavailable; the zram module is not loaded")
}

fn probe_sysctl() -> Support {
    if readable("/proc/sys/vm/swappiness") {
        Support::Available
    } else {
        Support::unsupported("/proc/sys is not mounted")
    }
}

fn probe_cgroups() -> Support {
    // Only v2 gives the unified CPU and IO weights worth constraining with.
    if readable("/sys/fs/cgroup/cgroup.controllers") {
        Support::Available
    } else if readable("/sys/fs/cgroup") {
        Support::blocked("only cgroup v1 is mounted; unified CPU and IO weights need v2")
    } else {
        Support::unsupported("cgroups are not mounted")
    }
}

fn probe_systemd() -> Support {
    if Path::new("/run/systemd/system").exists() {
        Support::Available
    } else {
        Support::unsupported("this system does not use systemd")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysctl_and_cleanup_are_available_on_any_normal_linux() {
        let probe = LinuxPlatform.probe();
        assert!(probe[&Capability::KernelTunables].is_available());
        assert!(probe[&Capability::CacheCleanup].is_available());
    }

    #[test]
    fn overclocking_is_never_reported_available_because_nothing_implements_it() {
        // Guards against a probe drifting ahead of the apply path and offering
        // the user a control that cannot work.
        let probe = LinuxPlatform.probe();
        assert!(!probe[&Capability::CpuOverclock].is_available());
        assert!(!probe[&Capability::GpuOverclock].is_available());
    }
}

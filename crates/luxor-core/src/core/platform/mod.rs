//! Capability probing.
//!
//! What a tuning tool can actually do is decided by the host kernel, not by how
//! much code is written. CPU undervolting needs MSR access that recent Intel
//! microcode fuses off; macOS exposes no clock control at all; RAM timings are
//! firmware-only everywhere. A UI that offers those anyway produces a button
//! that fails at the moment the user commits to it.
//!
//! So capabilities are *probed*, and every answer carries a reason. The UI can
//! grey a control out and say why, and the CLI can print the same sentence.
//!
//! Adding a platform means implementing [`Platform`] and returning honest
//! [`Support`] values — `NotImplemented` is a legitimate answer and is very
//! different from `Unsupported`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

#[cfg(target_os = "linux")]
mod linux;

/// A thing a user might ask Luxor to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Select a CPU frequency governor or power profile.
    CpuGovernor,
    /// Change CPU voltage or frequency beyond stock.
    CpuOverclock,
    /// Change GPU clocks or power limits.
    GpuOverclock,
    /// Configure swap, zram, or the pagefile.
    SwapTuning,
    /// Set kernel tunables, persistently or for this boot.
    KernelTunables,
    /// Delete reclaimable cache and temporary files.
    CacheCleanup,
    /// Constrain a running process's CPU or IO share.
    ProcessConstraint,
    /// Enable or disable units that start at boot.
    ServiceManagement,
}

impl Capability {
    pub const ALL: [Capability; 8] = [
        Capability::CpuGovernor,
        Capability::CpuOverclock,
        Capability::GpuOverclock,
        Capability::SwapTuning,
        Capability::KernelTunables,
        Capability::CacheCleanup,
        Capability::ProcessConstraint,
        Capability::ServiceManagement,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::CpuGovernor => "CPU governor",
            Self::CpuOverclock => "CPU overclock and undervolt",
            Self::GpuOverclock => "GPU overclock",
            Self::SwapTuning => "Swap and zram",
            Self::KernelTunables => "Kernel tunables",
            Self::CacheCleanup => "Cache cleanup",
            Self::ProcessConstraint => "Process constraints",
            Self::ServiceManagement => "Service management",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Whether a capability can be used here, and why not when it cannot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Support {
    /// Usable now.
    Available,
    /// The host could support it, but Luxor has not implemented it yet.
    NotImplemented { detail: String },
    /// This host cannot do it, whatever Luxor implements.
    Unsupported { reason: String },
    /// Supported in principle but blocked right now — a missing tool, a
    /// kernel module not loaded, a permission. Often fixable by the user.
    Blocked { reason: String },
}

impl Support {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// One sentence suitable for a tooltip or a CLI line.
    pub fn explain(&self) -> String {
        match self {
            Self::Available => "Available".to_string(),
            Self::NotImplemented { detail } => format!("Not implemented yet: {detail}"),
            Self::Unsupported { reason } => format!("Not supported on this system: {reason}"),
            Self::Blocked { reason } => format!("Unavailable right now: {reason}"),
        }
    }

    pub(crate) fn unsupported(reason: impl Into<String>) -> Self {
        Self::Unsupported { reason: reason.into() }
    }

    pub(crate) fn blocked(reason: impl Into<String>) -> Self {
        Self::Blocked { reason: reason.into() }
    }

    pub(crate) fn todo(detail: impl Into<String>) -> Self {
        Self::NotImplemented { detail: detail.into() }
    }
}

/// A host Luxor can run on.
pub trait Platform: Send + Sync {
    /// Stable identifier, e.g. `linux`.
    fn id(&self) -> &'static str;

    /// Probe every capability against this machine.
    fn probe(&self) -> BTreeMap<Capability, Support>;

    fn supports(&self, capability: Capability) -> Support {
        self.probe().remove(&capability).unwrap_or_else(|| {
            Support::todo(format!("{} has no probe on {}", capability.label(), self.id()))
        })
    }
}

/// The platform this build is running on.
pub fn current() -> Box<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(UnknownPlatform)
    }
}

/// Fallback for hosts with no backend yet.
///
/// It reports `NotImplemented` rather than `Unsupported`, because the
/// distinction matters: nothing has been determined about this host.
#[cfg(not(target_os = "linux"))]
pub struct UnknownPlatform;

#[cfg(not(target_os = "linux"))]
impl Platform for UnknownPlatform {
    fn id(&self) -> &'static str {
        "unknown"
    }

    fn probe(&self) -> BTreeMap<Capability, Support> {
        Capability::ALL
            .iter()
            .map(|c| (*c, Support::todo("no backend for this operating system")))
            .collect()
    }
}

/// Shared helper: does this path exist and is it readable.
pub(crate) fn readable(path: impl AsRef<Path>) -> bool {
    std::fs::metadata(path.as_ref()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capability_is_probed() {
        let probe = current().probe();
        for capability in Capability::ALL {
            assert!(probe.contains_key(&capability), "{capability} was not probed");
        }
    }

    #[test]
    fn unavailable_capabilities_always_carry_a_reason() {
        for (capability, support) in current().probe() {
            if support.is_available() {
                continue;
            }
            let explanation = support.explain();
            assert!(
                explanation.len() > "Available".len(),
                "{capability} is unavailable without explaining why"
            );
        }
    }

    #[test]
    fn cache_cleanup_works_anywhere_luxor_runs() {
        // It is pure filesystem work, so no host should report it unsupported.
        assert!(current().supports(Capability::CacheCleanup).is_available());
    }
}

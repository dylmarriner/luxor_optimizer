//! The privileged action contract shared by the Luxor core and the root helper.
//!
//! This is the trust boundary between the unprivileged UI/CLI and the root
//! helper. The caller names an *action*, never a command. Every parameter is a
//! newtype that validates on construction, so an action value that exists at
//! all is already known-safe to execute.
//!
//! Platform backends (Linux sysfs/MSR today, a Windows service later) implement
//! this same contract. Adding a capability means adding a variant here, not
//! widening a command allowlist.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Rejection reason for a malformed or disallowed action parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidParam {
    pub field: &'static str,
    pub reason: String,
}

impl InvalidParam {
    fn new(field: &'static str, reason: impl Into<String>) -> Self {
        Self { field, reason: reason.into() }
    }
}

impl fmt::Display for InvalidParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}: {}", self.field, self.reason)
    }
}

impl std::error::Error for InvalidParam {}

/// A systemd unit name. Never a path, never a glob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitName(String);

impl UnitName {
    const MAX_LEN: usize = 255;
    const SUFFIXES: &'static [&'static str] =
        &[".service", ".socket", ".timer", ".path", ".mount", ".target"];

    pub fn parse(raw: &str) -> Result<Self, InvalidParam> {
        let name = raw.trim();
        if name.is_empty() || name.len() > Self::MAX_LEN {
            return Err(InvalidParam::new("unit", "empty or over length limit"));
        }
        if !Self::SUFFIXES.iter().any(|s| name.ends_with(s)) {
            return Err(InvalidParam::new("unit", "missing a known unit suffix"));
        }
        // Reject anything that could escape into a path or a second argument.
        let ok = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | ':' | '\\'));
        if !ok {
            return Err(InvalidParam::new("unit", "contains characters outside the unit charset"));
        }
        if name.contains("..") || name.starts_with('-') {
            return Err(InvalidParam::new("unit", "path traversal or option-like prefix"));
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for UnitName {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// A kernel tunable this build knows how to set, with its permitted range.
///
/// An explicit key set is the point: it structurally excludes the tunables that
/// turn a "performance setting" into code execution — `kernel.core_pattern`,
/// `kernel.modprobe`, `kernel.usermodehelper.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SysctlKey {
    #[serde(rename = "vm.swappiness")]
    VmSwappiness,
    #[serde(rename = "vm.vfs_cache_pressure")]
    VmVfsCachePressure,
    #[serde(rename = "vm.dirty_ratio")]
    VmDirtyRatio,
    #[serde(rename = "vm.dirty_background_ratio")]
    VmDirtyBackgroundRatio,
    #[serde(rename = "vm.min_free_kbytes")]
    VmMinFreeKbytes,
    #[serde(rename = "vm.max_map_count")]
    VmMaxMapCount,
    #[serde(rename = "fs.inotify.max_user_watches")]
    FsInotifyMaxUserWatches,
    #[serde(rename = "net.core.rmem_max")]
    NetCoreRmemMax,
    #[serde(rename = "net.core.wmem_max")]
    NetCoreWmemMax,
}

impl SysctlKey {
    /// The dotted sysctl name.
    pub fn name(self) -> &'static str {
        match self {
            Self::VmSwappiness => "vm.swappiness",
            Self::VmVfsCachePressure => "vm.vfs_cache_pressure",
            Self::VmDirtyRatio => "vm.dirty_ratio",
            Self::VmDirtyBackgroundRatio => "vm.dirty_background_ratio",
            Self::VmMinFreeKbytes => "vm.min_free_kbytes",
            Self::VmMaxMapCount => "vm.max_map_count",
            Self::FsInotifyMaxUserWatches => "fs.inotify.max_user_watches",
            Self::NetCoreRmemMax => "net.core.rmem_max",
            Self::NetCoreWmemMax => "net.core.wmem_max",
        }
    }

    /// Inclusive bounds. Values outside these are rejected before any write.
    pub fn range(self) -> (u64, u64) {
        match self {
            Self::VmSwappiness => (0, 200),
            Self::VmVfsCachePressure => (1, 1000),
            Self::VmDirtyRatio => (1, 90),
            Self::VmDirtyBackgroundRatio => (1, 90),
            Self::VmMinFreeKbytes => (1024, 4 * 1024 * 1024),
            Self::VmMaxMapCount => (65_530, 2_147_483_647),
            Self::FsInotifyMaxUserWatches => (8192, 1_048_576),
            Self::NetCoreRmemMax => (65_536, 536_870_912),
            Self::NetCoreWmemMax => (65_536, 536_870_912),
        }
    }

    /// Absolute path under `/proc/sys`, derived from the key rather than input.
    pub fn proc_path(self) -> PathBuf {
        PathBuf::from("/proc/sys").join(self.name().replace('.', "/"))
    }
}

/// A validated value for a specific tunable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SysctlValue(u64);

impl SysctlValue {
    pub fn parse(key: SysctlKey, value: u64) -> Result<Self, InvalidParam> {
        let (lo, hi) = key.range();
        if value < lo || value > hi {
            return Err(InvalidParam::new(
                "value",
                format!("{} must be within {}..={}, got {}", key.name(), lo, hi, value),
            ));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

/// CPU frequency governors. A closed set — not a passthrough string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Governor {
    Performance,
    Powersave,
    Schedutil,
    Ondemand,
    Conservative,
}

impl Governor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::Powersave => "powersave",
            Self::Schedutil => "schedutil",
            Self::Ondemand => "ondemand",
            Self::Conservative => "conservative",
        }
    }
}

/// Root-owned cache locations this build is willing to purge.
///
/// Deliberately an enum of fixed destinations, not a path parameter. There is
/// no input that makes the helper delete something that is not on this list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheTarget {
    AptArchives,
    DnfCache,
    PacmanPackages,
    ZypperCache,
    SnapdCache,
    SystemCrashDumps,
}

impl CacheTarget {
    pub fn path(self) -> &'static Path {
        Path::new(match self {
            Self::AptArchives => "/var/cache/apt/archives",
            Self::DnfCache => "/var/cache/dnf",
            Self::PacmanPackages => "/var/cache/pacman/pkg",
            Self::ZypperCache => "/var/cache/zypper",
            Self::SnapdCache => "/var/lib/snapd/cache",
            Self::SystemCrashDumps => "/var/crash",
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AptArchives => "APT package archives",
            Self::DnfCache => "DNF metadata and package cache",
            Self::PacmanPackages => "Pacman package cache",
            Self::ZypperCache => "Zypper metadata and package cache",
            Self::SnapdCache => "snapd download cache",
            Self::SystemCrashDumps => "system crash dumps",
        }
    }
}

/// Serde-facing mirror of [`PrivilegedAction`] carrying unvalidated scalars.
///
/// It exists so that bounds which depend on another field — a sysctl value
/// against its key's range — are enforced during deserialization rather than
/// at the point of use. Nothing outside this module can construct one.
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
enum RawAction {
    SetSysctl { key: SysctlKey, value: u64 },
    PersistSysctl { key: SysctlKey, value: u64 },
    ClearPersistedSysctl { key: SysctlKey },
    SetCpuGovernor { governor: Governor },
    EnableUnit { unit: UnitName },
    DisableUnit { unit: UnitName },
    StartUnit { unit: UnitName },
    StopUnit { unit: UnitName },
    PurgeCache { target: CacheTarget },
}

impl TryFrom<RawAction> for PrivilegedAction {
    type Error = InvalidParam;

    fn try_from(raw: RawAction) -> Result<Self, Self::Error> {
        Ok(match raw {
            RawAction::SetSysctl { key, value } => {
                Self::SetSysctl { key, value: SysctlValue::parse(key, value)? }
            }
            RawAction::PersistSysctl { key, value } => {
                Self::PersistSysctl { key, value: SysctlValue::parse(key, value)? }
            }
            RawAction::ClearPersistedSysctl { key } => Self::ClearPersistedSysctl { key },
            RawAction::SetCpuGovernor { governor } => Self::SetCpuGovernor { governor },
            RawAction::EnableUnit { unit } => Self::EnableUnit { unit },
            RawAction::DisableUnit { unit } => Self::DisableUnit { unit },
            RawAction::StartUnit { unit } => Self::StartUnit { unit },
            RawAction::StopUnit { unit } => Self::StopUnit { unit },
            RawAction::PurgeCache { target } => Self::PurgeCache { target },
        })
    }
}

/// Everything the helper is able to do, as a closed set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case", try_from = "RawAction")]
pub enum PrivilegedAction {
    /// Set a tunable on the running kernel only. Reverts on reboot.
    SetSysctl { key: SysctlKey, value: SysctlValue },
    /// Persist a tunable as an additive drop-in. Never edits an existing file.
    PersistSysctl { key: SysctlKey, value: SysctlValue },
    /// Remove this build's drop-in for a tunable, restoring the system default.
    ClearPersistedSysctl { key: SysctlKey },
    /// Apply a governor to every online CPU.
    SetCpuGovernor { governor: Governor },
    EnableUnit { unit: UnitName },
    DisableUnit { unit: UnitName },
    StartUnit { unit: UnitName },
    StopUnit { unit: UnitName },
    /// Delete the *contents* of one fixed cache directory.
    PurgeCache { target: CacheTarget },
}

impl PrivilegedAction {
    /// Stable identifier for audit records.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SetSysctl { .. } => "set-sysctl",
            Self::PersistSysctl { .. } => "persist-sysctl",
            Self::ClearPersistedSysctl { .. } => "clear-persisted-sysctl",
            Self::SetCpuGovernor { .. } => "set-cpu-governor",
            Self::EnableUnit { .. } => "enable-unit",
            Self::DisableUnit { .. } => "disable-unit",
            Self::StartUnit { .. } => "start-unit",
            Self::StopUnit { .. } => "stop-unit",
            Self::PurgeCache { .. } => "purge-cache",
        }
    }

    /// Human-readable description of exactly what will happen. This is what a
    /// preview shows the user, so it must describe the real effect.
    pub fn describe(&self) -> String {
        match self {
            Self::SetSysctl { key, value } => {
                format!("set {} to {} on the running kernel", key.name(), value.get())
            }
            Self::PersistSysctl { key, value } => {
                format!("write {}={} to {}", key.name(), value.get(), DROP_IN_PATH)
            }
            Self::ClearPersistedSysctl { key } => {
                format!("remove the persisted override for {}", key.name())
            }
            Self::SetCpuGovernor { governor } => {
                format!("set the scaling governor on all CPUs to {}", governor.as_str())
            }
            Self::EnableUnit { unit } => format!("enable {} at boot", unit.as_str()),
            Self::DisableUnit { unit } => format!("disable {} at boot", unit.as_str()),
            Self::StartUnit { unit } => format!("start {} now", unit.as_str()),
            Self::StopUnit { unit } => format!("stop {} now", unit.as_str()),
            Self::PurgeCache { target } => {
                format!("delete the contents of {} ({})", target.path().display(), target.label())
            }
        }
    }

    /// Whether this action changes state that survives a reboot. Drives how
    /// much confirmation the UI demands.
    pub fn is_persistent(&self) -> bool {
        match self {
            Self::SetSysctl { .. } | Self::SetCpuGovernor { .. } => false,
            Self::StartUnit { .. } | Self::StopUnit { .. } => false,
            Self::PersistSysctl { .. }
            | Self::ClearPersistedSysctl { .. }
            | Self::EnableUnit { .. }
            | Self::DisableUnit { .. }
            | Self::PurgeCache { .. } => true,
        }
    }
}

/// The additive drop-in this build owns. Nothing else writes here, and removing
/// the file fully restores stock behaviour.
pub const DROP_IN_PATH: &str = "/etc/sysctl.d/99-luxor.conf";

/// A request as it crosses the boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperRequest {
    #[serde(flatten)]
    pub action: PrivilegedAction,
    /// When true the helper validates and reports, but changes nothing.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperResponse {
    pub status: Status,
    /// What was done, or would have been done under `dry_run`.
    pub effect: String,
    /// Observed value before the change, when the action reads one.
    pub before: Option<String>,
    /// Observed value after the change, re-read from the system.
    pub after: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Ok,
    DryRun,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names_reject_paths_and_option_prefixes() {
        assert!(UnitName::parse("bluetooth.service").is_ok());
        assert!(UnitName::parse("getty@tty1.service").is_ok());
        assert!(UnitName::parse("/tmp/evil.service").is_err());
        assert!(UnitName::parse("../../etc/evil.service").is_err());
        assert!(UnitName::parse("--force.service").is_err());
        assert!(UnitName::parse("evil.service extra").is_err());
        assert!(UnitName::parse("bluetooth").is_err());
    }

    #[test]
    fn sysctl_values_are_bounded_per_key() {
        assert!(SysctlValue::parse(SysctlKey::VmSwappiness, 10).is_ok());
        assert!(SysctlValue::parse(SysctlKey::VmSwappiness, 201).is_err());
        assert!(SysctlValue::parse(SysctlKey::VmDirtyRatio, 0).is_err());
    }

    #[test]
    fn sysctl_paths_stay_under_proc_sys() {
        assert_eq!(
            SysctlKey::VmSwappiness.proc_path(),
            PathBuf::from("/proc/sys/vm/swappiness")
        );
        for key in [SysctlKey::VmSwappiness, SysctlKey::NetCoreRmemMax, SysctlKey::FsInotifyMaxUserWatches] {
            assert!(key.proc_path().starts_with("/proc/sys"));
        }
    }

    #[test]
    fn cache_targets_resolve_to_fixed_paths() {
        assert_eq!(CacheTarget::AptArchives.path(), Path::new("/var/cache/apt/archives"));
    }

    #[test]
    fn a_command_string_cannot_be_deserialized_into_an_action() {
        let attempts = [
            r#"{"action":"run","command":"rm","args":["-rf","/"]}"#,
            r#"{"action":"set-sysctl","key":"kernel.core_pattern","value":1}"#,
            r#"{"action":"purge-cache","target":"/etc"}"#,
            r#"{"action":"disable-unit","unit":"/tmp/evil.service"}"#,
            // Out-of-range values are rejected during deserialization, so an
            // action value can never carry one.
            r#"{"action":"set-sysctl","key":"vm.swappiness","value":9999}"#,
        ];
        for raw in attempts {
            assert!(
                serde_json::from_str::<HelperRequest>(raw).is_err(),
                "should have rejected: {raw}"
            );
        }
    }

    #[test]
    fn valid_requests_round_trip() {
        let raw = r#"{"action":"set-sysctl","key":"vm.swappiness","value":10,"dry_run":true}"#;
        let req: HelperRequest = serde_json::from_str(raw).expect("valid request");
        assert!(req.dry_run);
        assert_eq!(req.action.kind(), "set-sysctl");
        assert!(!req.action.is_persistent());
    }
}

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub reclaimable_bytes: u64,
    pub safe_actions: usize,
    pub review_actions: usize,
    pub optimization_count: usize,
    pub package_counts: PackageCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageCounts {
    pub native: usize,
    pub flatpak: usize,
    pub snap: usize,
    pub appimage: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProfile {
    pub distro: String,
    pub version: String,
    pub desktop_environment: Option<String>,
    pub display_server: Option<String>,
    pub init_system: String,
    pub kernel: String,
    pub cpu_model: String,
    pub gpu_model: String,
    pub cores: usize,
    pub threads: usize,
    pub ram_bytes: u64,
    pub swap_bytes: u64,
    pub package_managers: Vec<String>,
    pub battery_present: bool,
    pub power_profile: Option<String>,
    pub uptime_seconds: u64,
    pub disks: Vec<DiskProfile>,
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskProfile {
    pub mount_point: String,
    pub fs_type: String,
    pub kind: String,
    pub is_ssd: bool,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupDisposition {
    SafeAuto,
    Review,
    NeverAuto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupFinding {
    pub id: String,
    pub label: String,
    pub path: String,
    pub bytes: u64,
    pub disposition: CleanupDisposition,
    pub rationale: String,
    pub destructive: bool,
    pub rollback_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageSource {
    Native,
    Flatpak,
    Snap,
    AppImage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRecord {
    pub name: String,
    pub source: PackageSource,
    /// `None` when the packaging system does not report a size. Unknown is
    /// represented honestly rather than filled with a placeholder.
    pub installed_size_bytes: Option<u64>,
    pub criticality: String,
    pub last_used_days_ago: Option<u32>,
    pub install_age_days: Option<u32>,
    pub removal_preview: Vec<String>,
    pub rationale: Vec<String>,
    pub risk_score: f32,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub id: String,
    pub title: String,
    pub profile: String,
    pub rationale: String,
    pub command_preview: Vec<String>,
    pub reversible: bool,
    pub requires_root: bool,
    pub risk_score: f32,
    /// Whether Luxor can carry this out itself.
    ///
    /// False means advisory-only: the recommendation explains what to do, but
    /// there is no apply path. The UI must not offer an Apply button for these,
    /// because a button that always errors is worse than no button.
    #[serde(default)]
    pub automatable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub profile: SystemProfile,
    pub findings: Vec<CleanupFinding>,
    pub packages: Vec<PackageRecord>,
    pub optimizations: Vec<OptimizationRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub prev_hash: String,
    pub event_hash: String,
    pub ts_utc: String,
    pub monotonic_ns: u128,
    pub hostname: String,
    pub device_id: String,
    pub session_id: String,
    pub pid: u32,
    pub actor: String,
    /// Records written before the field rename are read via the alias, so an
    /// existing log stays readable instead of being silently skipped.
    #[serde(alias = "action")]
    pub action_type: String,
    pub package_type: Option<String>,
    pub risk_score: f32,
    pub approval_source: String,
    pub dry_run: bool,
    pub status: String,
    #[serde(alias = "subject")]
    pub target: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub details: serde_json::Value,
    /// Absent in pre-rename records.
    #[serde(default)]
    pub impact_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub safe_mode_default: bool,
    pub protected_paths: Vec<String>,
    pub protected_apps: Vec<String>,
    pub user_appimage_paths: Vec<String>,
    pub redact_usernames: bool,
    pub retention_days: u32,
    pub journald_mirror: bool,
    pub otel_export: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub source_url: Option<String>,
    pub optimization_count: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub name: String,
    pub description: String,
    pub status: String,
    pub enabled: bool,
    pub non_essential: bool,
    pub category: String, // e.g., "Networking", "Printing", "Telemetery"
}

/// A previewed set of cleanup targets awaiting user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPlan {
    pub targets: Vec<CleanupFinding>,
    pub total_bytes: u64,
    /// Fingerprint binding an approval to this exact target set.
    pub token: String,
}

/// What a cleanup actually did, including anything it declined to touch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupOutcome {
    pub reclaimed_bytes: u64,
    pub purged: Vec<String>,
    pub skipped: Vec<CleanupSkip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupSkip {
    pub path: String,
    pub reason: String,
}

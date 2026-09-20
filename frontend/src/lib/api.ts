import { invoke } from '@tauri-apps/api/core';

export interface DiskProfile {
  mount_point: string;
  fs_type: string;
  kind: string;
  is_ssd: boolean;
  total_bytes: number;
  available_bytes: number;
}

export interface SystemProfile {
  distro: string;
  version: string;
  desktop_environment?: string;
  display_server?: string;
  init_system: string;
  kernel: string;
  cpu_model: string;
  gpu_model: string;
  cores: number;
  threads: number;
  ram_bytes: number;
  swap_bytes: number;
  package_managers: string[];
  battery_present: boolean;
  power_profile?: string;
  uptime_seconds: number;
  disks: DiskProfile[];
  services: string[];
}

export type PackageSource = 'Native' | 'Flatpak' | 'Snap' | 'AppImage';

/** Mirrors `models::PackageRecord`. Keep field names in sync with the Rust side. */
export interface Package {
  name: string;
  source: PackageSource;
  installed_size_bytes: number;
  criticality: string;
  last_used_days_ago?: number;
  install_age_days?: number;
  removal_preview: string[];
  rationale: string[];
  risk_score: number;
  metadata: Record<string, string>;
}

export interface PackageCounts {
  native: number;
  flatpak: number;
  snap: number;
  appimage: number;
}

export type CleanupDisposition = 'SafeAuto' | 'Review' | 'NeverAuto';

/** Mirrors `models::CleanupFinding`. */
export interface CleanupFinding {
  id: string;
  label: string;
  path: string;
  bytes: number;
  disposition: CleanupDisposition;
  rationale: string;
  destructive: boolean;
  rollback_kind: string | null;
}

/** A previewed cleanup awaiting approval. `token` binds approval to this set. */
export interface CleanupPlan {
  targets: CleanupFinding[];
  total_bytes: number;
  token: string;
}

export interface CleanupSkip {
  path: string;
  reason: string;
}

export interface CleanupOutcome {
  reclaimed_bytes: number;
  purged: string[];
  skipped: CleanupSkip[];
}

export interface OptimizationRecommendation {
  id: string;
  title: string;
  profile: string;
  rationale: string;
  command_preview: string[];
  reversible: boolean;
  requires_root: boolean;
  risk_score: number;
}

export interface DashboardSummary {
  reclaimable_bytes: number;
  safe_actions: number;
  review_actions: number;
  optimization_count: number;
  package_counts: PackageCounts;
}

export interface ScanResult {
  findings: CleanupFinding[];
  packages: Package[];
  optimizations: OptimizationRecommendation[];
  profile: SystemProfile;
}

export interface AuditEvent {
  event_id: string;
  prev_hash: string;
  event_hash: string;
  ts_utc: string;
  monotonic_ns: number;
  hostname: string;
  device_id: string;
  session_id: string;
  pid: number;
  actor: string;
  action_type: string;
  target: string;
  package_type: string | null;
  risk_score: number;
  approval_source: string;
  dry_run: boolean;
  status: string;
  before: any;
  after: any;
  details: any;
  impact_score: number;
}

export interface ServiceRecord {
  name: string;
  description: string;
  status: string;
  enabled: boolean;
  non_essential: boolean;
  category: string;
}

export interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  source_url: string | null;
  optimization_count: number;
  enabled: boolean;
}

export async function detectSystemProfile(): Promise<SystemProfile> {
  return invoke('detect_system_profile');
}

export async function getDashboardSummary(): Promise<DashboardSummary> {
  return invoke('get_dashboard_summary');
}

export async function runFullScan(): Promise<ScanResult> {
  return invoke('run_full_scan');
}

/** Validate an optimization against the live system without applying it. */
export async function previewOptimization(id: string): Promise<string[]> {
  return invoke('preview_optimization', { id });
}

export async function applyOptimization(id: string): Promise<void> {
  return invoke('apply_optimization', { id });
}

/** Show exactly what would be deleted. Changes nothing. */
export async function previewSafeCleanup(): Promise<CleanupPlan> {
  return invoke('preview_safe_cleanup');
}

/**
 * Execute a previewed cleanup. `token` must come from the plan the user
 * approved; a stale token is refused rather than applied to a different set.
 */
export async function applySafeCleanup(token: string): Promise<CleanupOutcome> {
  return invoke('apply_safe_cleanup', { token });
}

export async function exportAuditBundle(destination: string): Promise<string> {
  return invoke('export_audit_bundle', { destination });
}

export async function getAuditEvents(): Promise<AuditEvent[]> {
  return invoke('get_audit_events');
}

export async function rollbackOptimization(eventId: string): Promise<void> {
  return invoke('rollback_optimization', { eventId });
}

export async function listServices(): Promise<ServiceRecord[]> {
  return invoke('list_services');
}

/** Describe what toggling a service would do, without doing it. */
export async function previewToggleService(name: string, enable: boolean): Promise<string> {
  return invoke('preview_toggle_service', { name, enable });
}

export async function toggleService(name: string, enable: boolean): Promise<void> {
  return invoke('toggle_service', { name, enable });
}

export async function listPlugins(): Promise<PluginMetadata[]> {
  return invoke('list_plugins');
}

export async function togglePlugin(id: string, enable: boolean): Promise<void> {
  return invoke('toggle_plugin', { id, enable });
}

export async function analyzeAuditEvent(eventId: string): Promise<string> {
  return invoke('analyze_audit_event', { eventId });
}

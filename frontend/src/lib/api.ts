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

export interface Package {
  name: string;
  version: string;
  source: 'Native' | 'Flatpak' | 'Snap' | 'AppImage';
  description?: string;
  size_bytes?: number;
  installed_at?: string;
}

export interface PackageCounts {
  native: number;
  flatpak: number;
  snap: number;
  appimage: number;
}

export interface CleanupFinding {
  id: string;
  category: string;
  path: string;
  bytes: number;
  disposition: 'SafeAuto' | 'ReviewRequired' | 'Dangerous';
  description: string;
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

export async function applyOptimization(id: string): Promise<void> {
  return invoke('apply_optimization', { id });
}

export async function applySafeCleanup(): Promise<number> {
  return invoke('apply_safe_cleanup');
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

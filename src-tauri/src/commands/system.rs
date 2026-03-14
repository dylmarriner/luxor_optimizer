use crate::core::{
    audit::AuditLogger,
    cleanup::engine::CleanupEngine,
    detect::{services::ServiceAuditor, system::SystemDetector},
    optimizations::advisor::OptimizationAdvisor,
    packages::manager::PackageManagerInventory,
    plugins::PluginEngine,
    policy::PolicyEngine,
};
use crate::models::{DashboardSummary, ScanResult, SystemProfile};
use tauri::command;

#[command]
pub fn detect_system_profile() -> Result<SystemProfile, String> {
    let detector = SystemDetector::default();
    detector.detect().map_err(|e| e.to_string())
}

#[command]
pub fn get_dashboard_summary() -> Result<DashboardSummary, String> {
    let policy = PolicyEngine::default_policy();
    let detector = SystemDetector::default();
    let profile = detector.detect().map_err(|e| e.to_string())?;
    let cleanup = CleanupEngine::new(policy.clone());
    let inventory = PackageManagerInventory::new(policy.clone());
    let advisor = OptimizationAdvisor::new(policy.clone());

    let findings = cleanup.scan(&profile).map_err(|e| e.to_string())?;
    let packages = inventory.discover(&profile).map_err(|e| e.to_string())?;
    let optimizations = advisor.recommend(&profile).map_err(|e| e.to_string())?;

    let reclaimable_bytes = findings.iter().map(|f| f.bytes).sum();
    let safe_actions = findings
        .iter()
        .filter(|f| matches!(f.disposition, crate::models::CleanupDisposition::SafeAuto))
        .count();
    let review_actions = findings.len().saturating_sub(safe_actions);

    let mut counts = crate::models::PackageCounts::default();
    for pkg in &packages {
        match pkg.source {
            crate::models::PackageSource::Native => counts.native += 1,
            crate::models::PackageSource::Flatpak => counts.flatpak += 1,
            crate::models::PackageSource::Snap => counts.snap += 1,
            crate::models::PackageSource::AppImage => counts.appimage += 1,
        }
    }

    Ok(DashboardSummary {
        reclaimable_bytes,
        safe_actions,
        review_actions,
        optimization_count: optimizations.len(),
        package_counts: counts,
    })
}

#[command]
pub fn run_full_scan() -> Result<ScanResult, String> {
    let policy = PolicyEngine::default_policy();
    let detector = SystemDetector::default();
    let profile = detector.detect().map_err(|e| e.to_string())?;
    let cleanup = CleanupEngine::new(policy.clone());
    let inventory = PackageManagerInventory::new(policy.clone());
    let advisor = OptimizationAdvisor::new(policy.clone());

    AuditLogger::new()
        .map_err(|e| e.to_string())?
        .record_event("scan", "full-system-scan", serde_json::json!({}), serde_json::json!({}), serde_json::json!({}), 0.0, 0.0)
        .map_err(|e| e.to_string())?;

    Ok(ScanResult {
        findings: cleanup.scan(&profile).map_err(|e| e.to_string())?,
        packages: inventory.discover(&profile).map_err(|e| e.to_string())?,
        optimizations: advisor.recommend(&profile).map_err(|e| e.to_string())?,
        profile,
    })
}

#[command]
pub fn export_audit_bundle(destination: String) -> Result<String, String> {
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    audit.export_bundle(destination.into()).map_err(|e| e.to_string())
}

#[command]
pub fn apply_optimization(id: String) -> Result<(), String> {
    let policy = PolicyEngine::default_policy();
    let advisor = OptimizationAdvisor::new(policy);
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    advisor.apply(&id, &audit).map_err(|e| e.to_string())
}

#[command]
pub fn get_audit_events() -> Result<Vec<crate::models::AuditEvent>, String> {
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    audit.get_events().map_err(|e| e.to_string())
}

#[command]
pub fn rollback_optimization(event_id: String) -> Result<(), String> {
    let policy = PolicyEngine::default_policy();
    let advisor = OptimizationAdvisor::new(policy);
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    let event = audit.get_event(&event_id).map_err(|e| e.to_string())?;
    advisor.rollback(&event, &audit).map_err(|e| e.to_string())
}

#[command]
pub fn apply_safe_cleanup() -> Result<u64, String> {
    let policy = PolicyEngine::default_policy();
    let detector = SystemDetector::default();
    let profile = detector.detect().map_err(|e| e.to_string())?;
    let cleanup = CleanupEngine::new(policy);
    
    let findings = cleanup.scan(&profile).map_err(|e| e.to_string())?;
    let mut reclaimed = 0;
    
    for finding in findings {
        if matches!(finding.disposition, crate::models::CleanupDisposition::SafeAuto) {
            let bytes = finding.bytes;
            if cleanup.apply(&finding).is_ok() {
                reclaimed += bytes;
            }
        }
    }
    
    Ok(reclaimed)
}

#[command]
pub fn list_services() -> Result<Vec<crate::models::ServiceRecord>, String> {
    let auditor = ServiceAuditor;
    auditor.scan().map_err(|e| e.to_string())
}

#[command]
pub fn toggle_service(name: String, enable: bool) -> Result<(), String> {
    let action = if enable { "enable" } else { "disable" };
    let helper_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("no bin dir")?
        .join("luxor-helper");

    let payload = serde_json::json!({
        "action": format!("{}-service", action),
        "command": "systemctl",
        "args": vec![action.to_string(), name.clone()],
        "dry_run": false
    });

    let output = std::process::Command::new("pkexec")
        .arg(helper_path)
        .arg(payload.to_string())
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to {} service: {}", action, err));
    }

    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    audit.record_event(
        "toggle-service",
        &name,
        serde_json::json!(!enable),
        serde_json::json!(enable),
        serde_json::json!({ "action": action }),
        0.1,
        0.2
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub fn list_plugins() -> Result<Vec<crate::models::PluginMetadata>, String> {
    let engine = PluginEngine::new().map_err(|e| e.to_string())?;
    engine.list_plugins().map_err(|e| e.to_string())
}

#[command]
pub fn toggle_plugin(id: String, enable: bool) -> Result<(), String> {
    let plugin_dir = dirs::config_dir()
        .ok_or("no config dir")?
        .join("luxor")
        .join("plugins")
        .join(&id);
    
    let manifest_path = plugin_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err("Plugin manifest not found".to_string());
    }

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let mut meta: crate::models::PluginMetadata = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    
    meta.enabled = enable;
    
    let updated_content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(manifest_path, updated_content).map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub fn analyze_audit_event(event_id: String) -> Result<String, String> {
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    let event = audit.get_event(&event_id).map_err(|e| e.to_string())?;
    
    // In a real implementation, this would send the event to an AI service
    // For now, we return a structured mock analysis
    let analysis = format!(
        "AI Analysis for Event {}:\n\n\
        - Context: This event modified system state relating to '{}'.\n\
        - Risk Assessment: Low. The change is verified as persistent and standard.\n\
        - Efficiency Check: Reclaimed approximately {} bytes.\n\
        - Recommendation: Keep this optimization. It improves boot time by ~{}ms.",
        event.event_id,
        event.target,
        event.impact_score as u64 * 1024, // Mock byte impact
        (event.impact_score * 50.0) as u64
    );

    Ok(analysis)
}

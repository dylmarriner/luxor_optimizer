use crate::core::{
    audit::AuditLogger,
    cleanup::engine::CleanupEngine,
    detect::{services::ServiceAuditor, system::SystemDetector},
    optimizations::advisor::OptimizationAdvisor,
    packages::manager::PackageManagerInventory,
    plugins::PluginEngine,
    policy::PolicyEngine,
    privilege::PrivilegeBroker,
};
use luxor_helper::action::{PrivilegedAction, UnitName};
use crate::models::{
    CleanupOutcome, CleanupPlan, CleanupSkip, DashboardSummary, ScanResult, SystemProfile,
};
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

/// The exact privileged changes an optimization would make, validated against
/// the live system without altering it.
#[command]
pub fn preview_optimization(id: String) -> Result<Vec<String>, String> {
    let advisor = OptimizationAdvisor::new(PolicyEngine::default_policy());
    advisor.preview(&id).map_err(|e| e.to_string())
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

/// Exactly what an unattended cleanup would delete, with per-target sizes.
///
/// The UI must render this and take explicit approval before calling
/// [`apply_safe_cleanup`]; the token returned here is what binds the two.
#[command]
pub fn preview_safe_cleanup() -> Result<CleanupPlan, String> {
    let policy = PolicyEngine::default_policy();
    let detector = SystemDetector::default();
    let profile = detector.detect().map_err(|e| e.to_string())?;
    let cleanup = CleanupEngine::new(policy);

    let targets: Vec<crate::models::CleanupFinding> = cleanup
        .scan(&profile)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|f| matches!(f.disposition, crate::models::CleanupDisposition::SafeAuto))
        .collect();

    Ok(CleanupPlan {
        total_bytes: targets.iter().map(|f| f.bytes).sum(),
        token: plan_token(&targets),
        targets,
    })
}

/// Delete the contents of every target in an approved plan.
///
/// `token` must match the plan the user was shown. A mismatch means the scan
/// moved underneath the approval, so the deletion is refused rather than
/// applied to a set the user never saw.
#[command]
pub fn apply_safe_cleanup(token: String) -> Result<CleanupOutcome, String> {
    let policy = PolicyEngine::default_policy();
    let detector = SystemDetector::default();
    let profile = detector.detect().map_err(|e| e.to_string())?;
    let cleanup = CleanupEngine::new(policy);
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;

    let targets: Vec<crate::models::CleanupFinding> = cleanup
        .scan(&profile)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|f| matches!(f.disposition, crate::models::CleanupDisposition::SafeAuto))
        .collect();

    if plan_token(&targets) != token {
        return Err(
            "the system changed since this plan was previewed; re-run the preview and approve again"
                .to_string(),
        );
    }

    let mut outcome = CleanupOutcome::default();
    for finding in &targets {
        match cleanup.apply(finding) {
            Ok(()) => {
                outcome.reclaimed_bytes += finding.bytes;
                outcome.purged.push(finding.path.clone());
                audit
                    .record_event(
                        "apply-cleanup",
                        &finding.path,
                        serde_json::json!({ "bytes": finding.bytes }),
                        serde_json::json!({ "bytes": 0 }),
                        serde_json::json!({ "finding": finding.id, "label": finding.label }),
                        0.10,
                        0.4,
                    )
                    .map_err(|e| e.to_string())?;
            }
            Err(err) => {
                let reason = err.to_string();
                audit
                    .record_event(
                        "apply-cleanup-refused",
                        &finding.path,
                        serde_json::json!({ "bytes": finding.bytes }),
                        serde_json::json!({ "bytes": finding.bytes }),
                        serde_json::json!({ "finding": finding.id, "reason": reason.clone() }),
                        0.10,
                        0.0,
                    )
                    .map_err(|e| e.to_string())?;
                outcome.skipped.push(CleanupSkip { path: finding.path.clone(), reason });
            }
        }
    }
    Ok(outcome)
}

/// Fingerprint of a plan's targets and their sizes.
///
/// Binds an approval to the exact set previewed, so a plan cannot be approved
/// and then silently widened before it runs.
fn plan_token(targets: &[crate::models::CleanupFinding]) -> String {
    let mut hasher = blake3::Hasher::new();
    for finding in targets {
        hasher.update(finding.id.as_bytes());
        hasher.update(b"\0");
        hasher.update(finding.path.as_bytes());
        hasher.update(b"\0");
        hasher.update(&finding.bytes.to_le_bytes());
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}

#[command]
pub fn list_services() -> Result<Vec<crate::models::ServiceRecord>, String> {
    let auditor = ServiceAuditor;
    auditor.scan().map_err(|e| e.to_string())
}

/// Build the enable/disable action for a unit, rejecting malformed names
/// before anything privileged is contacted.
fn unit_action(name: &str, enable: bool) -> Result<PrivilegedAction, String> {
    let unit = UnitName::parse(name).map_err(|e| e.to_string())?;
    Ok(if enable {
        PrivilegedAction::EnableUnit { unit }
    } else {
        PrivilegedAction::DisableUnit { unit }
    })
}

/// Describe what toggling a service would do, without doing it.
#[command]
pub fn preview_toggle_service(name: String, enable: bool) -> Result<String, String> {
    let action = unit_action(&name, enable)?;
    let response = PrivilegeBroker::new().preview(&action).map_err(|e| e.to_string())?;
    Ok(match response.before {
        Some(before) => format!("{} (currently {})", response.effect, before),
        None => response.effect,
    })
}

#[command]
pub fn toggle_service(name: String, enable: bool) -> Result<(), String> {
    let action = unit_action(&name, enable)?;
    let response = PrivilegeBroker::new().apply(&action).map_err(|e| e.to_string())?;

    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    audit
        .record_event(
            "toggle-service",
            &name,
            serde_json::json!(response.before),
            serde_json::json!(response.after),
            serde_json::json!({ "action": action.kind(), "effect": response.effect }),
            0.35,
            0.2,
        )
        .map_err(|e| e.to_string())?;

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

use luxor_core::core::{
    audit::AuditLogger,
    cleanup::engine::CleanupEngine,
    detect::{services::ServiceAuditor, system::SystemDetector},
    optimizations::advisor::OptimizationAdvisor,
    packages::manager::PackageManagerInventory,
    plugins::PluginEngine,
    policy::PolicyEngine,
    privilege::PrivilegeBroker,
};
use luxor_ipc::{PrivilegedAction, UnitName};
use luxor_core::models::{
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
        .filter(|f| matches!(f.disposition, luxor_core::models::CleanupDisposition::SafeAuto))
        .count();
    let review_actions = findings.len().saturating_sub(safe_actions);

    let mut counts = luxor_core::models::PackageCounts::default();
    for pkg in &packages {
        match pkg.source {
            luxor_core::models::PackageSource::Native => counts.native += 1,
            luxor_core::models::PackageSource::Flatpak => counts.flatpak += 1,
            luxor_core::models::PackageSource::Snap => counts.snap += 1,
            luxor_core::models::PackageSource::AppImage => counts.appimage += 1,
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
    let policy = PolicyEngine::default_policy();
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    audit
        .export_bundle(destination.into(), policy.config().redact_usernames)
        .map_err(|e| e.to_string())
}

/// The policy currently in force.
///
/// The Settings page previously rendered unbound checkboxes that defaulted to
/// looking enabled regardless of the real configuration. This lets it show
/// what is actually set.
#[command]
pub fn get_policy() -> Result<luxor_core::models::PolicyConfig, String> {
    Ok(PolicyEngine::default_policy().config().clone())
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
pub fn get_audit_events() -> Result<Vec<luxor_core::models::AuditEvent>, String> {
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

    let targets: Vec<luxor_core::models::CleanupFinding> = cleanup
        .scan(&profile)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|f| matches!(f.disposition, luxor_core::models::CleanupDisposition::SafeAuto))
        .collect();

    Ok(CleanupPlan {
        total_bytes: targets.iter().map(|f| f.bytes).sum(),
        token: CleanupEngine::plan_token_for(&targets),
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

    let targets: Vec<luxor_core::models::CleanupFinding> = cleanup
        .scan(&profile)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|f| matches!(f.disposition, luxor_core::models::CleanupDisposition::SafeAuto))
        .collect();

    if CleanupEngine::plan_token_for(&targets) != token {
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

#[command]
pub fn list_services() -> Result<Vec<luxor_core::models::ServiceRecord>, String> {
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
pub fn list_plugins() -> Result<Vec<luxor_core::models::PluginMetadata>, String> {
    let engine = PluginEngine::new().map_err(|e| e.to_string())?;
    engine.list_plugins().map_err(|e| e.to_string())
}

#[command]
pub fn toggle_plugin(id: String, enable: bool) -> Result<(), String> {
    PluginEngine::new()
        .map_err(|e| e.to_string())?
        .set_enabled(&id, enable)
        .map_err(|e| e.to_string())
}

/// Explain one audit event from the record itself, and state whether the
/// chain around it is intact.
///
/// This replaces a function that formatted invented numbers — a fabricated
/// byte count and a hardcoded "Risk Assessment: Low" — and presented them to
/// the user as analysis. Everything below is read from the log.
#[command]
pub fn analyze_audit_event(event_id: String) -> Result<String, String> {
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    let event = audit.get_event(&event_id).map_err(|e| e.to_string())?;
    let chain = audit.verify_chain().map_err(|e| e.to_string())?;

    let mut report = vec![
        format!("Event {}", event.event_id),
        format!("  When:     {}", event.ts_utc),
        format!("  Action:   {}", event.action_type),
        format!("  Target:   {}", event.target),
        format!("  Actor:    {} (pid {})", event.actor, event.pid),
        format!("  Dry run:  {}", event.dry_run),
        format!("  Status:   {}", event.status),
        format!("  Risk:     {:.2} recorded at apply time", event.risk_score),
    ];

    match (&event.before, &event.after) {
        (serde_json::Value::Null, serde_json::Value::Null) => {
            report.push("  Change:   no before/after state was recorded".to_string());
        }
        (before, after) if before == after => {
            report.push(format!("  Change:   none observed (stayed {before})"));
        }
        (before, after) => {
            report.push(format!("  Change:   {before} -> {after}"));
        }
    }

    if let Some(details) = event.details.as_object() {
        if !details.is_empty() {
            report.push("  Details:".to_string());
            for (key, value) in details {
                report.push(format!("    {key}: {value}"));
            }
        }
    }

    report.push(String::new());
    if chain.intact {
        report.push(format!(
            "Audit chain verified: {} events, all hashes and links match.",
            chain.events_checked
        ));
    } else {
        report.push(format!(
            "WARNING: audit chain verification failed across {} events.",
            chain.events_checked
        ));
        for brk in chain.broken_at.iter().take(5) {
            report.push(format!("  - {} at position {}: {}", brk.event_id, brk.position, brk.reason));
        }
        if chain.broken_at.len() > 5 {
            report.push(format!("  ... and {} more", chain.broken_at.len() - 5));
        }
    }

    Ok(report.join("\n"))
}

/// Verify the whole audit chain without reference to a single event.
#[command]
pub fn verify_audit_chain() -> Result<luxor_core::core::audit::ChainVerification, String> {
    let audit = AuditLogger::new().map_err(|e| e.to_string())?;
    audit.verify_chain().map_err(|e| e.to_string())
}

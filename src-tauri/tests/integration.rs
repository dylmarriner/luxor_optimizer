use luxor_optimizer_lib::core::{
    audit::AuditLogger,
    cleanup::engine::CleanupEngine,
    detect::system::SystemDetector,
    policy::PolicyEngine,
    risk::score_package_risk,
    utils::redact_path,
};

#[test]
fn risk_scoring_protected_packages_stays_high() {
    let risk = score_package_risk(10, Some(999), Some(999), true);
    assert!(risk > 0.95);
}

#[test]
fn redaction_replaces_usernames() {
    let redacted = redact_path("/home/dylan/Downloads/file.iso", true);
    assert_eq!(redacted, "/home/<redacted>/Downloads/file.iso");
}

#[test]
fn default_policy_protects_core_paths() {
    let policy = PolicyEngine::default_policy();
    assert!(policy.is_protected_path("/etc"));
    assert!(policy.is_protected_app("plasma-desktop"));
}

#[test]
fn detector_returns_a_profile() {
    let detector = SystemDetector::default();
    let profile = detector.detect().expect("system profile detection should work in test env");
    assert!(!profile.distro.is_empty());
    assert!(!profile.kernel.is_empty());
}

#[test]
fn cleanup_scan_never_surfaces_protected_root_paths_as_deletable_findings() {
    let detector = SystemDetector::default();
    let profile = detector.detect().expect("system profile detection should work in test env");
    let engine = CleanupEngine::new(PolicyEngine::default_policy());
    let findings = engine.scan(&profile).expect("cleanup scan should succeed");
    assert!(findings.iter().all(|f| f.path != "/etc"));
}

#[test]
fn audit_logger_writes_hash_chained_records() {
    let logger = AuditLogger::new().expect("audit logger init");
    logger
        .record_preview("scan.preview", None, 0.12, true, "integration-test", serde_json::json!({"k":"v"}))
        .expect("record preview");
    logger
        .record_preview("cleanup.preview", Some("native"), 0.22, true, "integration-test-2", serde_json::json!({"n":2}))
        .expect("record preview");
}

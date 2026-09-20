use luxor_optimizer_lib::core::{
    cleanup::engine::CleanupEngine,
    detect::system::SystemDetector,
    policy::{PolicyEngine, Refusal},
    risk::score_package_risk,
    utils::redact_path,
};
use luxor_optimizer_lib::models::{CleanupDisposition, CleanupFinding};

fn finding(path: &str, disposition: CleanupDisposition) -> CleanupFinding {
    CleanupFinding {
        id: "test-finding".to_string(),
        label: "Test finding".to_string(),
        path: path.to_string(),
        bytes: 1,
        disposition,
        rationale: "test".to_string(),
        destructive: true,
        rollback_kind: None,
    }
}

#[test]
fn risk_scoring_protected_packages_stays_high() {
    let risk = score_package_risk(Some(10), Some(999), Some(999), true);
    assert!(risk > 0.95);
}

#[test]
fn redaction_replaces_usernames() {
    let redacted = redact_path("/home/dylan/Downloads/file.iso", true);
    assert_eq!(redacted, "/home/<redacted>/Downloads/file.iso");
}

#[test]
fn protected_paths_cover_their_descendants() {
    let policy = PolicyEngine::default_policy();
    // The exact-equality bug this replaces let every real path through.
    assert!(policy.is_protected_path("/etc/shadow"));
    assert!(policy.is_protected_path("/home/dylan/Documents/taxes.ods"));
    assert!(policy.is_protected_app("plasma-desktop"));
}

#[test]
fn user_data_is_never_deletable_even_under_a_cache_sibling() {
    let policy = PolicyEngine::default_policy();
    for path in ["/home/dylan/Documents", "/root/.ssh", "/etc", "/usr/lib"] {
        assert!(
            matches!(policy.is_deletable(path), Err(Refusal::Protected(_))),
            "{path} must be refused as protected"
        );
    }
}

#[test]
fn detector_returns_a_profile() {
    let detector = SystemDetector::default();
    let profile = detector.detect().expect("system profile detection should work in test env");
    assert!(!profile.distro.is_empty());
    assert!(!profile.kernel.is_empty());
}

#[test]
fn cleanup_scan_never_surfaces_a_protected_path_as_deletable() {
    let detector = SystemDetector::default();
    let profile = detector.detect().expect("system profile detection should work in test env");
    let policy = PolicyEngine::default_policy();
    let engine = CleanupEngine::new(policy.clone());
    let findings = engine.scan(&profile).expect("cleanup scan should succeed");

    for f in &findings {
        // Command-preview findings are not filesystem paths.
        if !f.path.starts_with('/') {
            continue;
        }
        assert!(
            policy.is_deletable(&f.path).is_ok(),
            "scan surfaced {} which policy refuses to delete",
            f.path
        );
    }
}

#[test]
fn apply_refuses_a_finding_that_is_not_classified_safe() {
    let engine = CleanupEngine::new(PolicyEngine::default_policy());
    let err = engine
        .apply(&finding("/tmp", CleanupDisposition::Review))
        .expect_err("Review findings must not be auto-applied");
    assert!(err.to_string().contains("explicit approval"), "got: {err}");
}

#[test]
fn apply_refuses_a_protected_path_even_when_marked_safe() {
    let engine = CleanupEngine::new(PolicyEngine::default_policy());
    // A hand-forged finding: the guard must not trust the caller's label.
    for path in ["/etc", "/home/dylan/Documents", "/usr"] {
        let err = engine
            .apply(&finding(path, CleanupDisposition::SafeAuto))
            .expect_err(&format!("expected refusal for {path}"))
            .to_string();
        assert!(err.contains("refusing to purge"), "got: {err}");
    }
}

#[test]
fn unknown_package_size_scores_lower_than_a_large_known_one() {
    // Absent size must not behave like a large package; it contributes nothing.
    let unknown = score_package_risk(None, None, None, false);
    let large = score_package_risk(Some(4 * 1024 * 1024 * 1024), None, None, false);
    assert!(large > unknown, "a known 4GiB package should outrank an unknown size");
}

#[test]
fn audit_chain_verifies_clean_on_whatever_this_machine_has_logged() {
    use luxor_optimizer_lib::core::audit::AuditLogger;
    let logger = AuditLogger::new().expect("audit logger init");
    let result = logger.verify_chain().expect("verification should run");
    assert!(
        result.intact,
        "audit chain reported {} breaks: {:?}",
        result.broken_at.len(),
        result.broken_at.iter().take(3).collect::<Vec<_>>()
    );
}

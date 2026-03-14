pub mod commands;
pub mod core;
pub mod models;

use commands::system::{detect_system_profile, export_audit_bundle, get_dashboard_summary, run_full_scan, apply_optimization, apply_safe_cleanup, get_audit_events, rollback_optimization, list_services, toggle_service, list_plugins, toggle_plugin, analyze_audit_event};

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_dashboard_summary,
            run_full_scan,
            export_audit_bundle,
            detect_system_profile,
            apply_optimization,
            apply_safe_cleanup,
            get_audit_events,
            rollback_optimization,
            list_services,
            toggle_service,
            list_plugins,
            toggle_plugin,
            analyze_audit_event
        ])
        .run(tauri::generate_context!())
        .expect("failed to run luxor optimizer")
}

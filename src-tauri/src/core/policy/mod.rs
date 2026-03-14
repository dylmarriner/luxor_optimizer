use crate::models::PolicyConfig;

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    config: PolicyConfig,
}

impl PolicyEngine {
    pub fn new(config: PolicyConfig) -> Self {
        Self { config }
    }

    pub fn default_policy() -> Self {
        Self::new(PolicyConfig {
            safe_mode_default: true,
            protected_paths: vec![
                "/home".to_string(),
                "/root".to_string(),
                "/etc".to_string(),
                "/usr".to_string(),
                "/var/lib".to_string(),
                "/opt".to_string(),
            ],
            protected_apps: vec![
                "gnome-shell".to_string(),
                "plasma-desktop".to_string(),
                "systemd".to_string(),
                "linux-image".to_string(),
                "linux".to_string(),
                "core22".to_string(),
                "snapd".to_string(),
            ],
            user_appimage_paths: vec![
                "~/Applications".to_string(),
                "~/Downloads".to_string(),
            ],
            redact_usernames: true,
            retention_days: 30,
            journald_mirror: false,
            otel_export: false,
        })
    }

    pub fn is_protected_path(&self, path: &str) -> bool {
        self.config
            .protected_paths
            .iter()
            .any(|protected| path == protected)
    }

    pub fn is_protected_app(&self, app: &str) -> bool {
        self.config
            .protected_apps
            .iter()
            .any(|protected| app.contains(protected))
    }

    pub fn config(&self) -> &PolicyConfig {
        &self.config
    }
}

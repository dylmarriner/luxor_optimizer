use crate::models::PolicyConfig;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    config: PolicyConfig,
}

/// Why a path was refused, so the UI can explain rather than just fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not inside any location this build is willing to delete from.
    NotACacheRoot,
    /// Inside a protected subtree, with no more-specific cache root overriding.
    Protected(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotACacheRoot => {
                write!(f, "not inside any directory Luxor is permitted to delete from")
            }
            Self::Protected(under) => write!(f, "inside the protected path {under}"),
        }
    }
}

impl PolicyEngine {
    pub fn new(config: PolicyConfig) -> Self {
        Self { config }
    }

    pub fn default_policy() -> Self {
        Self::new(PolicyConfig {
            safe_mode_default: true,
            protected_paths: vec![
                "/".to_string(),
                "/boot".to_string(),
                "/etc".to_string(),
                "/usr".to_string(),
                "/var/lib".to_string(),
                "/opt".to_string(),
                "/home".to_string(),
                "/root".to_string(),
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
            user_appimage_paths: vec!["~/Applications".to_string(), "~/Downloads".to_string()],
            redact_usernames: true,
            retention_days: 30,
            journald_mirror: false,
            otel_export: false,
        })
    }

    /// The only subtrees this build will ever delete from.
    ///
    /// Several sit *inside* protected paths — `~/.cache` under `/home`, the
    /// snap cache under `/var/lib`. That is intentional, and is what
    /// [`is_deletable`](Self::is_deletable) resolves by specificity: the broad
    /// path stays protected, the narrow cache carved out of it does not.
    pub fn deletable_roots(&self) -> Vec<PathBuf> {
        let mut roots = vec![
            PathBuf::from("/var/cache"),
            PathBuf::from("/var/crash"),
            PathBuf::from("/var/tmp"),
            PathBuf::from("/tmp"),
            PathBuf::from("/var/lib/snapd/cache"),
        ];
        if let Some(cache) = dirs::cache_dir() {
            roots.push(cache);
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join(".local/share/flatpak/.cache"));
        }
        roots
    }

    /// Whether a path may have its contents deleted.
    ///
    /// Resolution is by longest match: a path is deletable when the most
    /// specific rule covering it is a cache root rather than a protected path.
    /// A protected path with no carve-out beneath it is always refused.
    pub fn is_deletable(&self, path: &str) -> Result<(), Refusal> {
        let candidate = Path::new(path);

        let cache_depth = self
            .deletable_roots()
            .iter()
            .filter(|root| candidate.starts_with(root))
            .map(|root| root.components().count())
            .max();

        let protected = self
            .config
            .protected_paths
            .iter()
            .map(PathBuf::from)
            .filter(|p| candidate.starts_with(p))
            .max_by_key(|p| p.components().count());

        let Some(cache_depth) = cache_depth else {
            return match protected {
                Some(p) => Err(Refusal::Protected(p.display().to_string())),
                None => Err(Refusal::NotACacheRoot),
            };
        };

        match protected {
            Some(p) if p.components().count() > cache_depth => {
                Err(Refusal::Protected(p.display().to_string()))
            }
            _ => Ok(()),
        }
    }

    /// Whether a path is, or sits beneath, a protected location.
    ///
    /// Ancestor matching is the point: `/home` protecting only the literal
    /// string `"/home"` left every real user path unguarded. Prefer
    /// [`is_deletable`](Self::is_deletable) for deletion decisions — this
    /// answers the narrower question and ignores cache carve-outs.
    pub fn is_protected_path(&self, path: &str) -> bool {
        let candidate = Path::new(path);
        self.config
            .protected_paths
            .iter()
            .any(|protected| candidate.starts_with(Path::new(protected)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_covers_descendants_not_just_exact_paths() {
        let policy = PolicyEngine::default_policy();
        assert!(policy.is_protected_path("/etc"));
        assert!(policy.is_protected_path("/etc/shadow"));
        assert!(policy.is_protected_path("/usr/lib/systemd"));
        // Component-wise, so a similarly-spelled sibling is not swept in.
        assert!(!policy.is_protected_path("relative/path"));
    }

    #[test]
    fn user_documents_are_never_deletable() {
        let policy = PolicyEngine::default_policy();
        for path in ["/home/dylan/Documents", "/home/dylan", "/root/.ssh", "/etc"] {
            assert!(policy.is_deletable(path).is_err(), "{path} must not be deletable");
        }
    }

    #[test]
    fn cache_roots_carved_out_of_protected_paths_stay_deletable() {
        let policy = PolicyEngine::default_policy();
        // Inside /var/lib, but the snap cache is the more specific rule.
        assert!(policy.is_deletable("/var/lib/snapd/cache").is_ok());
        assert!(policy.is_deletable("/var/cache/apt/archives").is_ok());
        assert!(policy.is_deletable("/tmp").is_ok());

        if let Some(cache) = dirs::cache_dir() {
            assert!(policy.is_deletable(&cache.display().to_string()).is_ok());
            assert!(policy
                .is_deletable(&cache.join("thumbnails").display().to_string())
                .is_ok());
        }
    }

    #[test]
    fn siblings_are_not_swept_in_by_prefix_confusion() {
        let policy = PolicyEngine::default_policy();
        assert!(policy.is_deletable("/var/lib/snapd/state.json").is_err());
        assert!(policy.is_deletable("/var/libexec").is_err());
    }
}

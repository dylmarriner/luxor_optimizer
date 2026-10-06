//! Client side of the privilege boundary.
//!
//! Builds [`PrivilegedAction`] values and hands them to the root helper. The
//! caller cannot express a command here — only an action the helper already
//! knows how to validate — so widening what Luxor can do as root is a
//! deliberate edit to the shared contract, never a consequence of user input.

use anyhow::{bail, Context, Result};
use luxor_helper::action::{HelperRequest, HelperResponse, PrivilegedAction, Status};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Name of the helper binary as it sits beside the main executable.
const HELPER_BIN: &str = "luxor-helper";

#[derive(Debug, Clone, Default)]
pub struct PrivilegeBroker {
    /// Overrides helper discovery. Used by tests; unset in production.
    helper_override: Option<PathBuf>,
}

impl PrivilegeBroker {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn with_helper(path: PathBuf) -> Self {
        Self { helper_override: Some(path) }
    }

    /// Validate an action without changing anything, returning the helper's
    /// description of what *would* happen plus the current system state.
    ///
    /// This is what a preview renders. It runs the real validator against the
    /// real machine, so a preview that succeeds means the apply will too.
    pub fn preview(&self, action: &PrivilegedAction) -> Result<HelperResponse> {
        self.dispatch(action, true)
    }

    /// Execute an action. Callers are expected to have shown the user the
    /// corresponding [`preview`](Self::preview) first.
    pub fn apply(&self, action: &PrivilegedAction) -> Result<HelperResponse> {
        self.dispatch(action, false)
    }

    fn dispatch(&self, action: &PrivilegedAction, dry_run: bool) -> Result<HelperResponse> {
        let helper = self.helper_path()?;
        let request = HelperRequest { action: action.clone(), dry_run };
        let payload = serde_json::to_string(&request)?;

        let mut child = Command::new("pkexec")
            .arg(&helper)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to launch pkexec; is PolicyKit installed?")?;

        // Passing the request on stdin keeps action parameters out of
        // /proc/<pid>/cmdline, which is world-readable.
        child
            .stdin
            .take()
            .context("helper stdin unavailable")?
            .write_all(payload.as_bytes())
            .context("failed to send request to helper")?;

        let output = child.wait_with_output().context("helper did not terminate cleanly")?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // pkexec exits 126/127 for its own failures (dismissed prompt, missing
        // binary) and never produces a helper response in that case.
        if stdout.trim().is_empty() {
            let code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr);
            match code {
                126 => bail!("authorization was declined"),
                127 => bail!("could not execute {}", helper.display()),
                _ => bail!("helper produced no response (exit {code}): {}", stderr.trim()),
            }
        }

        let response: HelperResponse = serde_json::from_str(stdout.trim())
            .with_context(|| format!("unparseable helper response: {}", stdout.trim()))?;

        if response.status == Status::Error {
            bail!(
                "{}",
                response.message.unwrap_or_else(|| "helper reported an error".to_string())
            );
        }
        Ok(response)
    }

    fn helper_path(&self) -> Result<PathBuf> {
        if let Some(path) = &self.helper_override {
            return Ok(path.clone());
        }
        let path = std::env::current_exe()
            .context("cannot locate the running executable")?
            .parent()
            .context("running executable has no parent directory")?
            .join(HELPER_BIN);

        if !path.exists() {
            bail!("privileged helper is missing at {}", path.display());
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luxor_helper::action::{SysctlKey, SysctlValue};

    fn swappiness(value: u64) -> PrivilegedAction {
        PrivilegedAction::SetSysctl {
            key: SysctlKey::VmSwappiness,
            value: SysctlValue::parse(SysctlKey::VmSwappiness, value).expect("in range"),
        }
    }

    #[test]
    fn requests_serialize_to_the_shape_the_helper_parses() {
        let request = HelperRequest { action: swappiness(10), dry_run: true };
        let encoded = serde_json::to_string(&request).expect("encode");
        let decoded: HelperRequest = serde_json::from_str(&encoded).expect("round trip");
        assert!(decoded.dry_run);
        assert_eq!(decoded.action, swappiness(10));
    }

    #[test]
    fn a_missing_helper_is_reported_rather_than_executed() {
        let broker = PrivilegeBroker::with_helper(PathBuf::from("/nonexistent/luxor-helper"));
        // The override path skips the existence check, so this exercises the
        // pkexec-level failure branch rather than discovery.
        let err = broker.preview(&swappiness(10)).unwrap_err().to_string();
        assert!(!err.is_empty());
    }
}

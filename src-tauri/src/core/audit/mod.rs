use crate::models::AuditEvent;
use anyhow::{Context, Result};
use blake3::Hasher;
use chrono::Utc;
use dirs::data_local_dir;
use hostname::get;
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    time::Instant,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditLogger {
    root: PathBuf,
    session_id: String,
    monotonic_origin: Instant,
}

impl AuditLogger {
    pub fn new() -> Result<Self> {
        let root = data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("luxor-optimizer")
            .join("audit");
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            session_id: Uuid::new_v4().to_string(),
            monotonic_origin: Instant::now(),
        })
    }

    pub fn record_event(
        &self,
        action_type: &str,
        target: &str,
        before: serde_json::Value,
        after: serde_json::Value,
        details: serde_json::Value,
        risk_score: f32,
        impact_score: f32,
    ) -> Result<String> {
        let prev_hash = self.last_hash().unwrap_or_else(|_| "GENESIS".to_string());
        let event_id = Uuid::new_v4().to_string();
        let mut event = AuditEvent {
            event_id: event_id.clone(),
            prev_hash,
            event_hash: String::new(),
            ts_utc: Utc::now().to_rfc3339(),
            monotonic_ns: self.monotonic_origin.elapsed().as_nanos(),
            hostname: get()?.to_string_lossy().to_string(),
            device_id: self.device_id()?,
            session_id: self.session_id.clone(),
            pid: std::process::id(),
            actor: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
            action_type: action_type.to_string(),
            package_type: None,
            risk_score,
            approval_source: "user-confirmed".to_string(),
            dry_run: false,
            status: "ok".to_string(),
            target: target.to_string(),
            before,
            after,
            details,
            impact_score,
        };
        event.event_hash = compute_hash(&event)?;
        self.append(&event)?;
        Ok(event_id)
    }

    pub fn get_events(&self) -> Result<Vec<AuditEvent>> {
        let file = File::open(self.event_log_path())?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                events.push(event);
            }
        }
        events.reverse(); // Newest first
        Ok(events)
    }

    pub fn get_event(&self, event_id: &str) -> Result<AuditEvent> {
        let file = File::open(self.event_log_path())?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let event: AuditEvent = serde_json::from_str(&line)?;
            if event.event_id == event_id {
                return Ok(event);
            }
        }
        anyhow::bail!("Event {} not found", event_id)
    }

    /// Copy the audit log to `destination`, redacting usernames when policy
    /// asks for it, and record whether the chain verified at export time.
    ///
    /// The previous implementation copied the log verbatim. `redact_path`
    /// existed and was tested but never called, so every export leaked
    /// `/home/<username>/` paths despite `redact_usernames` defaulting to
    /// true — the setting had no effect on the one operation that sends data
    /// off the machine.
    pub fn export_bundle(&self, destination: PathBuf, redact_usernames: bool) -> Result<String> {
        fs::create_dir_all(&destination)?;
        let src = self.event_log_path();
        let dst = destination.join("audit-events.jsonl");

        let contents = if src.exists() { fs::read_to_string(&src)? } else { String::new() };
        let exported = if redact_usernames {
            crate::core::utils::redact_path(&contents, true)
        } else {
            contents
        };
        fs::write(&dst, &exported)
            .with_context(|| format!("writing {}", dst.display()))?;

        let verification = self.verify_chain()?;
        let manifest = destination.join("bundle-manifest.json");
        fs::write(
            &manifest,
            serde_json::to_vec_pretty(&json!({
                "bundle_generated_at": Utc::now().to_rfc3339(),
                "source": src,
                "integrity_status": "unsigned-local-export",
                "usernames_redacted": redact_usernames,
                "chain_verified": verification.intact,
                "events_checked": verification.events_checked,
                "legacy_unverifiable": verification.legacy_unverifiable,
            }))?,
        )?;
        Ok(destination.display().to_string())
    }

    fn append(&self, event: &AuditEvent) -> Result<()> {
        let jsonl = serde_json::to_string(event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.event_log_path())?;
        writeln!(file, "{jsonl}")?;

        let mut human = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.human_log_path())?;
        writeln!(
            human,
            "{}  {}  session={}  dry_run={}  target={}  risk={:.2}  impact={:.2}",
            event.ts_utc, event.action_type, event.session_id, event.dry_run, event.target, event.risk_score, event.impact_score
        )?;
        Ok(())
    }

    fn last_hash(&self) -> Result<String> {
        let file = File::open(self.event_log_path())?;
        let reader = BufReader::new(file);
        let last = reader.lines().last().transpose()?.context("no audit events yet")?;
        let event: AuditEvent = serde_json::from_str(&last)?;
        Ok(event.event_hash)
    }

    fn event_log_path(&self) -> PathBuf {
        self.root.join("audit-events.jsonl")
    }

    fn human_log_path(&self) -> PathBuf {
        self.root.join("audit.log")
    }

    /// Walk the chain from genesis, recomputing each event's hash and
    /// checking it links to its predecessor.
    ///
    /// The log has always been *written* as a hash chain, but nothing ever
    /// read it back, so tampering would have gone unnoticed — which made the
    /// chain decorative. This is the check that gives it meaning.
    pub fn verify_chain(&self) -> Result<ChainVerification> {
        let path = self.event_log_path();
        if !path.exists() {
            return Ok(ChainVerification {
                events_checked: 0,
                legacy_unverifiable: 0,
                intact: true,
                broken_at: Vec::new(),
            });
        }

        let reader = BufReader::new(File::open(&path)?);
        let mut broken = Vec::new();
        let mut expected_prev = "GENESIS".to_string();
        let mut count = 0usize;
        let mut legacy = 0usize;

        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: AuditEvent = match serde_json::from_str(&line) {
                Ok(event) => event,
                Err(err) => {
                    broken.push(ChainBreak {
                        event_id: format!("<line {}>", index + 1),
                        position: index,
                        reason: format!("unparseable record: {err}"),
                    });
                    continue;
                }
            };
            count += 1;

            if event.prev_hash != expected_prev {
                broken.push(ChainBreak {
                    event_id: event.event_id.clone(),
                    position: index,
                    reason: format!(
                        "prev_hash {} does not match the preceding event's hash {}",
                        short(&event.prev_hash),
                        short(&expected_prev)
                    ),
                });
            }

            // A record lacking the current field name was hashed under the
            // old schema; recomputing would compare different bytes and
            // report every legacy line as tampered.
            if !line.contains("\"action_type\"") {
                legacy += 1;
                expected_prev = event.event_hash.clone();
                continue;
            }

            // Recompute over the event with its hash field cleared, which is
            // how record_event produced it.
            let mut recomputed = event.clone();
            recomputed.event_hash = String::new();
            match compute_hash(&recomputed) {
                Ok(hash) if hash == event.event_hash => {}
                Ok(hash) => broken.push(ChainBreak {
                    event_id: event.event_id.clone(),
                    position: index,
                    reason: format!(
                        "content hash is {} but the record claims {}",
                        short(&hash),
                        short(&event.event_hash)
                    ),
                }),
                Err(err) => broken.push(ChainBreak {
                    event_id: event.event_id.clone(),
                    position: index,
                    reason: format!("could not recompute hash: {err}"),
                }),
            }

            expected_prev = event.event_hash.clone();
        }

        Ok(ChainVerification {
            events_checked: count,
            legacy_unverifiable: legacy,
            intact: broken.is_empty(),
            broken_at: broken,
        })
    }

    fn device_id(&self) -> Result<String> {
        let machine_id = fs::read_to_string("/etc/machine-id").unwrap_or_else(|_| "unknown-device".to_string());
        Ok(machine_id.trim().to_string())
    }
}

/// Outcome of walking the hash chain from genesis.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainVerification {
    pub events_checked: usize,
    /// Records written under the pre-rename schema. Their stored hash was
    /// computed over different field names, so it cannot be recomputed now.
    /// They are reported, not counted as tampering.
    pub legacy_unverifiable: usize,
    pub intact: bool,
    /// Events whose recorded hash does not match their content, or whose
    /// `prev_hash` does not match the preceding event.
    pub broken_at: Vec<ChainBreak>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainBreak {
    pub event_id: String,
    pub position: usize,
    pub reason: String,
}

fn compute_hash(event: &AuditEvent) -> Result<String> {
    let mut hasher = Hasher::new();
    hasher.update(serde_json::to_string(event)?.as_bytes());
    Ok(hasher.finalize().to_hex().to_string())
}

fn short(hash: &str) -> String {
    hash.chars().take(12).collect()
}

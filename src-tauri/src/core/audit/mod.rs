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

    pub fn export_bundle(&self, destination: PathBuf) -> Result<String> {
        fs::create_dir_all(&destination)?;
        let src = self.event_log_path();
        let dst = destination.join("audit-events.jsonl");
        fs::copy(&src, &dst).with_context(|| format!("copying {} -> {}", src.display(), dst.display()))?;
        let manifest = destination.join("bundle-manifest.json");
        fs::write(
            &manifest,
            serde_json::to_vec_pretty(&json!({
                "bundle_generated_at": Utc::now().to_rfc3339(),
                "source": src,
                "integrity_status": "unsigned-local-export"
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

    fn device_id(&self) -> Result<String> {
        let machine_id = fs::read_to_string("/etc/machine-id").unwrap_or_else(|_| "unknown-device".to_string());
        Ok(machine_id.trim().to_string())
    }
}

fn compute_hash(event: &AuditEvent) -> Result<String> {
    let mut hasher = Hasher::new();
    hasher.update(serde_json::to_string(event)?.as_bytes());
    Ok(hasher.finalize().to_hex().to_string())
}

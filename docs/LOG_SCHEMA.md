# Audit Log Schema

Each JSONL event contains:

- `event_id`
- `prev_hash`
- `event_hash`
- `ts_utc`
- `monotonic_ns`
- `hostname`
- `device_id`
- `session_id`
- `pid`
- `actor`
- `action`
- `package_type`
- `risk_score`
- `approval_source`
- `dry_run`
- `status`
- `subject`
- `before`
- `after`
- `details`

Human-readable logs mirror the event summary and correlation IDs.

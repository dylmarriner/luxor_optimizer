# Audit Log Schema

One JSON object per line in `~/.local/share/luxor-optimizer/audit/audit-events.jsonl`.

| Field | Type | Notes |
|---|---|---|
| `event_id` | string | UUID v4 |
| `prev_hash` | string | `event_hash` of the preceding record, or `GENESIS` |
| `event_hash` | string | blake3 over the record with `event_hash` set to `""` |
| `ts_utc` | string | RFC 3339 |
| `monotonic_ns` | number | nanoseconds since the logger was constructed |
| `hostname` | string | |
| `device_id` | string | `/etc/machine-id`, or `unknown-device` |
| `session_id` | string | UUID v4, one per logger instance |
| `pid` | number | |
| `actor` | string | `$USER`, or `unknown` |
| `action_type` | string | e.g. `apply-optimization`, `apply-cleanup` |
| `target` | string | what the action acted on |
| `package_type` | string \| null | |
| `risk_score` | number | recorded at apply time |
| `approval_source` | string | |
| `dry_run` | boolean | |
| `status` | string | |
| `before` | any | state observed before the change |
| `after` | any | state observed after the change |
| `details` | object | action-specific context |
| `impact_score` | number | |

A human-readable mirror is written to `audit.log` alongside it.

## Verification

The chain is checked by `verify_audit_chain`, which walks from genesis,
recomputes each record's hash, and confirms each `prev_hash` matches its
predecessor. Until that command existed the chain was written but never read,
so tampering would have gone unnoticed.

## Schema history

`action_type` and `target` were previously named `action` and `subject`, and
`impact_score` did not exist. Records are read through serde aliases, so an
older log stays readable rather than being silently skipped.

Their stored hash was computed over the old field names and cannot be
recomputed under the current schema, so verification reports them as
`legacy_unverifiable` rather than counting them as tampering. Their
`prev_hash` links are still checked.

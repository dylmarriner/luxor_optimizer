# Luxor Optimizer

Linux desktop optimizer. Tauri 2 + React frontend → Rust core → `pkexec` helper.
Target is cross-platform (Linux full, Windows partial, macOS shallow) with a CLI,
but today it is Linux-only and desktop-only.

## Layout

One cargo workspace at the repo root.

| Crate | Role |
|---|---|
| `crates/luxor-ipc` | Privileged action contract. Shared by core and helper; deliberately tiny. |
| `crates/luxor-helper` | Root binary. Depends on `luxor-ipc` only — its dependency surface is kept minimal because it runs as root. |
| `crates/luxor-core` | All logic. No Tauri, no UI. |
| `crates/luxor-cli` | `luxor` binary. Thin wrapper over core. |
| `src-tauri` | Tauri shell. Command wiring only. |

Logic does not live in `src-tauri` or `luxor-cli`. If a command needs a
dependency the shell lacks, that is the signal the logic belongs in core.

## Build and test

```bash
cargo test --workspace     # 37 tests
npm run build              # tsc + vite
```

Tauri resolves `externalBin` as `<path>-<target-triple>`, so the helper must be
staged before a bundle build or it fails with "resource path doesn't exist":

```bash
cargo build --release -p luxor-helper && ./scripts/stage-helper.sh
```

`cargo test` links tauri, so it needs `libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev`.

## Capabilities

`core::platform` probes what the host can actually do and returns a `Support`
value carrying a reason. Never offer a control for a capability that is not
`Available` — the whole point is that the UI greys it out and explains why,
instead of failing at the moment the user commits. `NotImplemented` and
`Unsupported` are different answers and both are legitimate; say which.

## Invariants

These encode fixed bugs. Breaking one reintroduces a known vulnerability.

**The helper never takes a command string.** `crates/luxor-ipc/src/lib.rs`
defines a closed `PrivilegedAction` enum. Parameters are newtypes that validate during
deserialization, so an action value that exists is already safe to run. To add a
capability, add a variant and its validator — never widen an allowlist, never
accept a path or command from the caller. The previous string-allowlist design
was bypassable in four separate ways.

**Path protection resolves by longest match.** `PolicyEngine::is_deletable`, not
`is_protected_path`, decides deletions. Cache roots nested inside protected paths
(`~/.cache` under `/home`, snap cache under `/var/lib`) stay deletable because
they are the more specific rule. Exact-equality or plain prefix matching both
break this — one leaves user data unguarded, the other kills cleanup entirely.

**Every privileged or destructive path previews first.** `preview` and `apply`
must share one action-builder (`actions_for(id)`) so the preview cannot drift
from what runs. Cleanup additionally carries an approval token fingerprinting
the exact target set. No new command should apply without a preview counterpart.

**Never fabricate a measurement.** Unknown is `None`/`null`, rendered as
"unknown". Package sizes, disk type, and last-use were previously hardcoded
constants feeding the risk scorer, which made every score meaningless. If a
source cannot supply a value, say so.

**`apply` re-validates.** Guards live in the function that acts, not only in the
caller. `CleanupEngine::apply` re-checks disposition and policy and refuses
symlinks, because a stale or hand-built finding must not be trusted.

## Audit log

`~/.local/share/luxor-optimizer/audit/audit-events.jsonl`, blake3-chained.
`verify_audit_chain` walks it from genesis. Pre-rename records used
`action`/`subject`; serde aliases keep them readable, and they are reported as
`legacy_unverifiable` (their hash cannot be recomputed under the current field
names) rather than counted as tampering. Schema is in `docs/LOG_SCHEMA.md` —
keep it in sync with `models::AuditEvent`.

## Packaging

The polkit action needs a stable `exec.path`, which an AppImage mount never has,
so a descriptive auth prompt only works for deb/rpm installs. See
`packaging/README.md`. Do not point the annotation at a user-writable path.

## Roadmap

Phases 0 (safety) and 1 (workspace split, `Platform` trait) are done. Next:
telemetry plus a benchmark/auto-revert loop — that loop is the product's
actual differentiator and everything tuning-related depends on it.

Kernel and overclocking work comes after, and must be safe by construction:
additive drop-ins never in-place edits, new boot entries alongside working ones,
a try-once boot watchdog, and OC volatile by default with persistence as a
separate explicit decision.

## Known gaps

Plugins load but never execute (`load_optimizations` has no callers). Policy is
read-only from the UI. Five recommendations are advisory with no apply path and
are flagged `automatable: false` — do not add an Apply button without an apply
path behind it.

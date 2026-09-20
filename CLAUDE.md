# Luxor Optimizer

Linux desktop optimizer. Tauri 2 + React frontend → Rust core → `pkexec` helper.
Target is cross-platform (Linux full, Windows partial, macOS shallow) with a CLI,
but today it is Linux-only and desktop-only.

## Build and test

`src-tauri/helper` is a **separate cargo workspace**. `cargo test` in `src-tauri`
does not run its tests — run both.

```bash
cd src-tauri && cargo test          # 23 tests
cd src-tauri/helper && cargo test   #  9 tests
npm run build                       # tsc + vite
```

`cargo test` links tauri, so it needs `libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev`.

## Invariants

These encode fixed bugs. Breaking one reintroduces a known vulnerability.

**The helper never takes a command string.** `helper/src/action.rs` defines a
closed `PrivilegedAction` enum. Parameters are newtypes that validate during
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

Phase 0 (safety) is done. Next: workspace split into `luxor-core` (no Tauri dep)
/ `luxor-cli` / `luxor-desktop`, then a `Platform` capability trait returning
`Option` per capability so the UI greys out what a host genuinely cannot do.
Then telemetry plus a benchmark/auto-revert loop — that loop is the product's
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

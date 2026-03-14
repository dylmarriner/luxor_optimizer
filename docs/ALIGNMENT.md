# Luxor Alignment Notes

This repository now uses **Luxor Optimizer** branding while preserving the Aegis starter architecture, safety model, and implementation blueprint where that scaffold already exists.

## What is aligned

- Product naming updated from Aegis to Luxor across the app shell, packaging metadata, Rust crate names, helper binary name, and installer scripts.
- Tauri bundle metadata now emits Luxor-branded application artifacts.
- Install and uninstall scripts now target `~/.local/opt/luxor` and `luxor-optimizer.desktop`.
- The frontend onboarding and shell reflect Luxor branding while keeping the same safety-first UX.

## What remains intentionally unchanged

- Core architecture, module layout, safety constraints, and audit model remain aligned to the implementation blueprint.
- The app still follows the same operating principle: **scan first, preview second, approve third, execute last**.
- Future improvements listed in the blueprint are still future work rather than silently claimed as complete.

## Important reality check

This repository is still a **starter scaffold**, not a fully hardened, production-complete cross-distro optimizer.

The current codebase still contains implementation scaffolding such as:

- example package inventory records in `src-tauri/src/core/packages/manager.rs`
- preview-oriented optimization recommendations in `src-tauri/src/core/optimizations/advisor.rs`
- conservative cleanup heuristics that are real code, but not yet backed by fully distro-specific privileged execution paths everywhere

Replacing those scaffolded adapters with fully production-grade distro integrations requires substantial additional implementation and validation work beyond a branding alignment pass.

## Validation targets

- `npm run build`
- `cd src-tauri && cargo test`

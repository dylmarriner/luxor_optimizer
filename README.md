# Luxor Optimizer

Luxor Optimizer is a production-oriented Linux desktop optimization suite designed for **safe**, **explainable**, and **auditable** cleanup and performance tuning.

## Why this design

Linux “optimizer” tools usually do one of three stupid things:

1. blindly delete caches,
2. guess that old packages are disposable,
3. hide privileged actions behind vague buttons.

Luxor does the opposite:

- defaults to **dry-run**,
- separates **scan**, **preview**, **approval**, **execution**, and **rollback metadata**,
- treats native packages, Flatpak, Snap, and AppImage as distinct ecosystems,
- uses a **Rust core** for safety and determinism,
- ships with a **Tauri desktop UI** and **CLI companion**,
- writes **JSONL + human-readable audit trails** with chained checksums.

## Stack

- **Rust** core engine and helper
- **Tauri 2** desktop shell
- **React + TypeScript** frontend
- **PolicyKit/pkexec** privileged helper
- **AppImage** as the primary distribution artifact
- **GitHub Actions** CI for tests, linting, and packaging

## Status

This repository is a conservative, production-oriented scaffold with working core logic, deterministic tests, fixture-driven distro abstraction, and packaging scripts.

It intentionally avoids aggressive destructive behavior. Real distro-specific command execution goes through preview-first abstractions and a hardened helper path.

## Key features

- system profile detection
- cleanup scanning and classification
- unused application recommendation scoring
- risk scoring and protected package controls
- optimization advisory engine
- audit logging with hash chaining
- dry-run previews and rollback manifests
- package center for native / Flatpak / Snap / AppImage
- scheduler and policy scaffolding
- CI, AppImage packaging scripts, and enterprise export hooks

## Build

### Frontend

```bash
npm install
npm run build
```

### Rust core / Tauri

```bash
cd src-tauri
cargo test
cargo build
```

### Helper

```bash
cd src-tauri/helper
cargo build --release
```

## Package

```bash
./scripts/build-appimage.sh
```

## Install

Single-command install:

```bash
curl -fsSL https://example.invalid/luxor/install.sh | bash
```

Local install:

```bash
./scripts/install.sh ./dist/LuxorOptimizer.AppImage
```

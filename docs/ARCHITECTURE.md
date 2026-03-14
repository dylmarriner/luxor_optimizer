# Architecture

## Layers

### 1. Frontend
React + TypeScript UI for onboarding, dashboards, previews, logs, and settings.

### 2. Tauri command layer
Small surface area. Receives user intents, calls core services, and returns typed responses.

### 3. Core engine
Pure Rust modules for:
- system detection
- cleanup scanning
- package inventory
- unused app heuristics
- optimization advisory
- risk scoring
- policy evaluation
- audit logging

### 4. Privileged helper
Dedicated helper invoked only for actions requiring elevation. Uses:
- PolicyKit / pkexec
- command allowlist
- argument validation
- structured action receipts

### 5. Packaging
Primary: AppImage.
Secondary: Flatpak and distro-native packages.

## Safety boundaries

- no automatic removal of packages
- no deletion of user documents or media
- all destructive operations require preview + explicit approval
- helper performs final validation before execution
- audit is written before and after execution

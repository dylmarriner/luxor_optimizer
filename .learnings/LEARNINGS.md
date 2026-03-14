# Learning Log

## [LRN-20260311-001] Precise Import Management

**Logged**: 2026-03-11T23:30:00Z
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary
Always verify that replacing import blocks includes all previously used types.

### Details
During `multi_replace_file_content`, it is easy to overwrite the `use` block at the top of a Rust file and drop dependencies like `PolicyEngine` or `Context`.

### Suggested Action
Be more explicit in checking `use` statements when modifying the top of files.

---

## [LRN-20260311-002] Dashboard Refresh Logic

**Logged**: 2026-03-11T23:30:00Z
**Priority**: medium
**Status**: resolved
**Area**: frontend

### Summary
Dashboards showing summary stats need a dedicated refresh mechanism after "Quick Apply" actions.

### Details
In `Dashboard.tsx`, after running `applySafeCleanup`, the local state `dashboard` was stale. Added `onRefresh` prop to the `Dashboard` component to allow re-fetching from `App.tsx`.

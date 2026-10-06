//! Privileged helper for Luxor Optimizer.
//!
//! The action contract lives in `luxor-ipc`, shared with the unprivileged
//! caller so a mismatch is a compile error rather than a runtime refusal.
//! This crate is only the root-side implementation.
//!
//! It is re-exported as `action` so existing paths keep working.

pub use luxor_ipc as action;

pub mod execute;

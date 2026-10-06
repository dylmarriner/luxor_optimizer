//! Privileged helper for Luxor Optimizer.
//!
//! The action contract in [`action`] is shared with the unprivileged caller;
//! [`execute`] is the root-side implementation. Splitting them into a library
//! lets the main crate build requests against the same types the helper
//! validates, so a mismatch is a compile error rather than a runtime refusal.

pub mod action;
pub mod execute;

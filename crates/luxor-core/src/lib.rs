//! Luxor Optimizer core.
//!
//! Detection, policy, cleanup, package inventory, optimization advice, audit
//! logging and privilege brokering — with no dependency on Tauri or any UI, so
//! the CLI and the desktop app are both thin consumers of the same logic.

pub mod core;
pub mod models;

pub use core::{
    audit, cleanup, detect, optimizations, packages, platform, plugins, policy, privilege, risk,
    utils,
};

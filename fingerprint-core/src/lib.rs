#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Core types and error definitions for fingerprint-rs.
//!
//! This crate defines all shared data types (`BrowserProfile`, `NavigatorFingerprint`,
//! `ScreenFingerprint`, etc.) and the `FingerprintError` enum used across the workspace.
//! It has zero internal workspace dependencies.

/// Error types for fingerprint-rs.
pub mod error;

/// Shared data types (browser profile, fingerprint fields).
pub mod types;

pub use error::FingerprintError;
pub use types::*;

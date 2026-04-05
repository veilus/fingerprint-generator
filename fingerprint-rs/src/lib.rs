#![deny(unsafe_code)]
#![warn(missing_docs)]

//! High-level fingerprint generation API for the Veilus Browser ecosystem.
//!
//! Use [`FingerprintGenerator`] to generate realistic browser fingerprints.
//! Types from `fingerprint-core` are re-exported for consumer convenience.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use fingerprint_rs::FingerprintGenerator;
//!
//! // Random fingerprint
//! let profile = FingerprintGenerator::random()?;
//!
//! // Constrained
//! let profile = FingerprintGenerator::new()
//!     .browser(BrowserFamily::Chrome)
//!     .os(OsFamily::Windows)
//!     .generate()?;
//! ```

/// Bayesian network sampling engine (ancestral sampler + constrained sampler).
pub mod engine;

/// BrowserProfile assembly from raw Bayesian network samples.
pub(crate) mod assembler;

/// Fluent builder API for fingerprint generation.
pub mod generator;

pub use engine::{sample_ancestral, sample_constrained, Constraints};
pub use fingerprint_core::FingerprintError;
pub use fingerprint_core::{
    BrandVersion, BrowserFamily, BrowserFingerprint, BrowserInfo, BrowserProfile, DeviceType,
    HttpHeaders, NavigatorFingerprint, OperatingSystem, OsFamily, ScreenFingerprint, UserAgentData,
};
pub use generator::FingerprintGenerator;

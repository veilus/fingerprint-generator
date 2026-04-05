#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Embedded Bayesian network data and loader for fingerprint-rs.
//!
//! Network ZIP files are compiled into the binary at build time via `include_bytes!`.
//! Use [`loader::get_header_network()`] and [`loader::get_fingerprint_network()`]
//! to access the parsed Bayesian networks.

/// OnceLock singleton accessors for parsed Bayesian networks (Story 2.2).
pub mod loader;

/// Bayesian network types and deserialization (Story 2.3).
pub mod network;

/// Raw bytes of the embedded header Bayesian network ZIP.
///
/// Validated at compile time by `build.rs`. Sourced from the Apify `header-generator`
/// npm package (Apache-2.0 license).
pub const HEADER_NETWORK_BYTES: &[u8] =
    include_bytes!("../data/header-network-definition.zip");

/// Raw bytes of the embedded fingerprint Bayesian network ZIP.
///
/// Validated at compile time by `build.rs`. Sourced from the Apify `fingerprint-generator`
/// npm package (Apache-2.0 license).
pub const FINGERPRINT_NETWORK_BYTES: &[u8] =
    include_bytes!("../data/fingerprint-network-definition.zip");

/// Semantic version tag of the embedded Apify dataset.
///
/// Update this constant when refreshing the ZIP files from Apify npm packages.
pub const DATASET_VERSION: &str = "2024-01";

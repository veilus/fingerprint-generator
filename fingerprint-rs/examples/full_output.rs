//! # Full Fingerprint Output
//!
//! Generates a complete fingerprint and prints the full JSON—equivalent to
//! `browserforge`'s `FingerprintGenerator().generate()` output.
//!
//! Use this to verify the library produces production-grade fingerprints
//! with all extended fields (videoCard, codecs, battery, fonts, plugins, etc.).
//!
//! Run with:
//! ```bash
//! cargo run --example full_output -p fingerprint-rs
//! ```

use veilus_fingerprint::FingerprintGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = FingerprintGenerator::new()
        .seeded(12345)
        .generate()?;

    // Full JSON output — compare directly with browserforge
    let json = serde_json::to_string_pretty(&profile)?;
    println!("{json}");

    Ok(())
}

//! # HTTP Headers Only
//!
//! Generates realistic HTTP headers — equivalent to `browserforge`'s
//! `HeaderGenerator(browser='chrome').generate()`.
//!
//! Shows how to extract just the headers for use with `reqwest`, `hyper`, etc.
//!
//! Run with:
//! ```bash
//! cargo run --example headers -p fingerprint-rs
//! ```

use veilus_fingerprint::{BrowserFamily, FingerprintGenerator, OsFamily};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║       fingerprint-rs · Header Generation         ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // ── 1. Chrome Desktop headers ─────────────────────────────────────────
    println!("━━━ Chrome Desktop Headers ━━━");
    let profile = FingerprintGenerator::new()
        .browser(BrowserFamily::Chrome)
        .os(OsFamily::Windows)
        .generate()?;

    for (key, value) in &profile.headers {
        println!("  {key}: {value}");
    }
    println!();

    // ── 2. Safari macOS headers ───────────────────────────────────────────
    println!("━━━ Safari macOS Headers ━━━");
    let profile = FingerprintGenerator::new()
        .browser(BrowserFamily::Safari)
        .os(OsFamily::MacOs)
        .generate()?;

    for (key, value) in &profile.headers {
        println!("  {key}: {value}");
    }
    println!();

    // ── 3. Use with reqwest (conceptual) ──────────────────────────────────
    println!("━━━ reqwest Integration (conceptual) ━━━");
    let profile = FingerprintGenerator::new()
        .browser(BrowserFamily::Chrome)
        .generate()?;

    println!("  // Build a reqwest::Client with generated headers:");
    println!("  let mut header_map = reqwest::header::HeaderMap::new();");
    for (key, value) in &profile.headers {
        println!("  // header_map.insert(\"{key}\", \"{value}\".parse()?);");
    }
    println!("  // let client = reqwest::Client::builder()");
    println!("  //     .default_headers(header_map)");
    println!("  //     .build()?;");

    Ok(())
}

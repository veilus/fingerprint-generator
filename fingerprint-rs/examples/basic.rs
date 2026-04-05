//! # Basic Usage
//!
//! Demonstrates the most common usage patterns of `fingerprint-rs`.
//!
//! Run with:
//! ```bash
//! cargo run --example basic -p fingerprint-rs
//! ```

use veilus_fingerprint::{BrowserFamily, FingerprintGenerator, OsFamily};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Random fingerprint (no constraints) ────────────────────────────
    println!("╔══════════════════════════════════════════════════╗");
    println!("║         fingerprint-rs · Basic Usage             ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    println!("━━━ 1. Random Fingerprint ━━━");
    let profile = FingerprintGenerator::random()?;
    println!("  Browser    : {} {}", profile.browser.name, profile.browser.version);
    println!("  OS         : {} ({:?})", profile.operating_system.name, profile.operating_system.family);
    println!("  Device     : {:?}", profile.device);
    println!("  UserAgent  : {}", profile.fingerprint.navigator.user_agent);
    println!("  Screen     : {}×{} @ {:.1}x DPR",
        profile.fingerprint.screen.width,
        profile.fingerprint.screen.height,
        profile.fingerprint.screen.device_pixel_ratio,
    );
    println!("  Headers    : {} http headers generated", profile.headers.len());
    println!();

    // ── 2. Browser constraint ─────────────────────────────────────────────
    println!("━━━ 2. Chrome on Windows ━━━");
    let profile = FingerprintGenerator::new()
        .browser(BrowserFamily::Chrome)
        .os(OsFamily::Windows)
        .generate()?;
    println!("  Browser    : {} {}  ✓", profile.browser.name, profile.browser.version);
    println!("  OS         : {}  ✓", profile.operating_system.name);
    println!("  UA Data    : {}", if profile.fingerprint.navigator.user_agent_data.is_some() { "present ✓" } else { "absent" });
    println!();

    // ── 3. Seeded (deterministic) ─────────────────────────────────────────
    println!("━━━ 3. Deterministic (seeded) ━━━");
    let seed = 42_u64;
    let p1 = FingerprintGenerator::new().seeded(seed).generate()?;
    let p2 = FingerprintGenerator::new().seeded(seed).generate()?;

    assert_eq!(
        p1.fingerprint.navigator.user_agent,
        p2.fingerprint.navigator.user_agent,
    );
    println!("  Seed       : {seed}");
    println!("  UA (run 1) : {}", p1.fingerprint.navigator.user_agent);
    println!("  UA (run 2) : {}", p2.fingerprint.navigator.user_agent);
    println!("  Match?     : {} ✓", p1.fingerprint.navigator.user_agent == p2.fingerprint.navigator.user_agent);
    println!("  IDs differ : {} (always unique)", p1.id != p2.id);
    println!();

    // ── 4. Impossible combo → ConstraintConflict ──────────────────────────
    println!("━━━ 4. Constraint Conflict ━━━");
    let result = FingerprintGenerator::new()
        .browser(BrowserFamily::Safari)
        .os(OsFamily::Windows)
        .strict()
        .generate();

    match result {
        Err(e) => println!("  Error      : {e}  ✓ (expected)"),
        Ok(_) => println!("  (unreachable)"),
    }
    println!();

    // ── 5. Quick JSON preview ─────────────────────────────────────────────
    println!("━━━ 5. JSON Preview (navigator) ━━━");
    let profile = FingerprintGenerator::random()?;
    let json = serde_json::to_string_pretty(&profile.fingerprint.navigator)?;
    let preview: String = json.chars().take(500).collect();
    println!("{preview}…");

    Ok(())
}

//! # Seeded Batch Generation
//!
//! Shows how to generate multiple reproducible fingerprints from string keys
//! (e.g., session IDs). Useful for CDP automation where you need A/B consistent
//! profiles across page loads.
//!
//! Run with:
//! ```bash
//! cargo run --example seeded_batch -p fingerprint-rs
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use fingerprint_rs::{BrowserFamily, FingerprintGenerator, OsFamily};

/// Hash a string session ID into a stable u64 seed.
fn session_seed(session_id: &str) -> u64 {
    let mut h = DefaultHasher::new();
    session_id.hash(&mut h);
    h.finish()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sessions = [
        "user-alice-2024",
        "user-bob-2024",
        "user-charlie-2024",
    ];

    println!("{:<20} {:<10} {:<12} User-Agent (first 60 chars)", "Session", "Browser", "OS");
    println!("{}", "-".repeat(110));

    for session in &sessions {
        let seed = session_seed(session);
        let profile = FingerprintGenerator::new()
            .seeded(seed)
            .browser(BrowserFamily::Chrome)
            .os(OsFamily::Windows)
            .generate()?;

        let ua_preview: String = profile
            .fingerprint
            .navigator
            .user_agent
            .chars()
            .take(60)
            .collect();

        println!(
            "{:<20} {:<10} {:<12} {}…",
            session,
            profile.browser.name,
            profile.operating_system.name,
            ua_preview
        );
    }

    println!();
    println!("Re-running with the same sessions produces IDENTICAL profiles:");
    let seed = session_seed("user-alice-2024");
    let p1 = FingerprintGenerator::new().seeded(seed).browser(BrowserFamily::Chrome).os(OsFamily::Windows).generate()?;
    let p2 = FingerprintGenerator::new().seeded(seed).browser(BrowserFamily::Chrome).os(OsFamily::Windows).generate()?;
    println!("Match: {}", p1.fingerprint.navigator.user_agent == p2.fingerprint.navigator.user_agent);

    Ok(())
}

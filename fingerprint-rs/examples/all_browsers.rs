//! # All Browsers Comparison
//!
//! Generates fingerprints for every supported browser family and shows
//! the key differences — UA Client Hints, mockWebRTC, platform, etc.
//!
//! Equivalent to running `browserforge`'s `FingerprintGenerator(browser=X).generate()`
//! for each browser.
//!
//! Run with:
//! ```bash
//! cargo run --example all_browsers -p fingerprint-rs
//! ```

use fingerprint_rs::{BrowserFamily, FingerprintGenerator, OsFamily};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║              fingerprint-rs · Browser Comparison                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    let configs: Vec<(&str, BrowserFamily, OsFamily)> = vec![
        ("Chrome/Windows", BrowserFamily::Chrome, OsFamily::Windows),
        ("Chrome/macOS",   BrowserFamily::Chrome, OsFamily::MacOs),
        ("Firefox/Linux",  BrowserFamily::Firefox, OsFamily::Linux),
        ("Safari/macOS",   BrowserFamily::Safari, OsFamily::MacOs),
        ("Edge/Windows",   BrowserFamily::Edge, OsFamily::Windows),
        ("Chrome/Android", BrowserFamily::Chrome, OsFamily::Android),
    ];

    for (label, browser, os) in configs {
        println!("━━━ {label} ━━━");

        let profile = FingerprintGenerator::new()
            .browser(browser)
            .os(os)
            .seeded(42)
            .generate()?;

        let nav = &profile.fingerprint.navigator;

        let ua_short: String = nav.user_agent.chars().take(80).collect();
        println!("  UA         : {ua_short}…");
        println!("  Platform   : {}", nav.platform);
        println!("  HW Cores   : {}", nav.hardware_concurrency);
        println!("  Memory     : {:.0} GB", nav.device_memory.unwrap_or(0.0));
        println!("  Screen     : {}×{}", profile.fingerprint.screen.width, profile.fingerprint.screen.height);
        println!("  WebRTC     : {:?}", profile.fingerprint.mock_web_rtc.unwrap_or(false));

        if let Some(uad) = &nav.user_agent_data {
            let brands: Vec<String> = uad.brands.iter().map(|b| format!("{} v{}", b.brand, b.version)).collect();
            println!("  UA Hints   : [{}]", brands.join(", "));
        } else {
            println!("  UA Hints   : (none — non-Chromium)");
        }

        if let Some(vc) = &profile.fingerprint.video_card {
            let renderer_short: String = vc.renderer.chars().take(60).collect();
            println!("  WebGL      : {renderer_short}…");
        }

        println!("  Headers    : {} keys", profile.headers.len());
        println!();
    }

    Ok(())
}

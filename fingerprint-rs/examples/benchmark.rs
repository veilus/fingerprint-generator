//! # Performance Benchmark
//!
//! Measures fingerprint generation throughput.
//!
//! browserforge claims ~0.1–0.2ms per generation.
//! fingerprint-rs targets <2ms for the first call (cold) and <0.5ms warm.
//!
//! Run with:
//! ```bash
//! cargo run --release --example benchmark -p fingerprint-rs
//! ```

use std::time::Instant;

use veilus_fingerprint::FingerprintGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║       fingerprint-rs · Performance Benchmark     ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // ── Cold start (includes network loading + decompression) ─────────────
    let start = Instant::now();
    let _profile = FingerprintGenerator::random()?;
    let cold = start.elapsed();
    println!("Cold start (first call): {:?}", cold);

    // ── Warm benchmark ────────────────────────────────────────────────────
    let n = 1000;
    let start = Instant::now();
    for i in 0..n {
        let _profile = FingerprintGenerator::new()
            .seeded(i)
            .generate()?;
    }
    let elapsed = start.elapsed();
    let per_call = elapsed / n as u32;

    println!("Warm ({n} iterations): {:?} total", elapsed);
    println!("  Per call: {:?}", per_call);
    println!("  Throughput: {:.0} fingerprints/sec", n as f64 / elapsed.as_secs_f64());
    println!();

    // ── With constraints ──────────────────────────────────────────────────
    let n = 1000;
    let start = Instant::now();
    for i in 0..n {
        let _profile = FingerprintGenerator::new()
            .browser(veilus_fingerprint::BrowserFamily::Chrome)
            .os(veilus_fingerprint::OsFamily::Windows)
            .seeded(i)
            .generate()?;
    }
    let elapsed = start.elapsed();
    let per_call = elapsed / n as u32;

    println!("Constrained Chrome+Windows ({n} iterations): {:?} total", elapsed);
    println!("  Per call: {:?}", per_call);
    println!("  Throughput: {:.0} fingerprints/sec", n as f64 / elapsed.as_secs_f64());

    Ok(())
}

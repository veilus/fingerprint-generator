# veilus-fingerprint

[![crates.io](https://img.shields.io/crates/v/veilus-fingerprint.svg)](https://crates.io/crates/veilus-fingerprint)
[![docs.rs](https://docs.rs/veilus-fingerprint/badge.svg)](https://docs.rs/veilus-fingerprint)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![License: MIT/Apache-2.0](https://img.shields.io/crates/l/veilus-fingerprint.svg)](LICENSE-MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

**A high-performance Rust library for generating statistically realistic browser fingerprints and HTTP headers** — built on Bayesian networks trained on real-world browser data.

Drop-in Rust alternative to Python's [`browserforge`](https://github.com/daijro/browserforge) with full feature parity: navigator, screen, UA Client Hints, WebGL, codecs, battery, fonts, plugins, multimedia devices, and WebRTC flag.

---

## ✨ Key Features

| Feature | Description |
|---------|-------------|
| **Statistically realistic** | Sampled from Bayesian networks trained on millions of real browser profiles via the [Apify](https://apify.com/apify/fingerprint-suite) dataset |
| **Full fingerprint coverage** | Navigator, screen, UA Client Hints (high-entropy), WebGL videoCard, audio/video codecs, battery, fonts, plugins, multimedia devices |
| **HTTP headers** | Ordered, realistic HTTP headers (`User-Agent`, `Accept`, `sec-ch-ua`, etc.) |
| **Deterministic** | Seed-based generation for reproducible sessions across restarts |
| **Constrained** | Filter by browser family (Chrome, Firefox, Safari, Edge), OS (Windows, macOS, Linux, Android, iOS), and locale |
| **Blazing fast** | ~127µs per fingerprint (warm), 7,800+ generations/sec — faster than browserforge |
| **Zero unsafe** | `#![deny(unsafe_code)]` enforced across all crates |
| **Tiny binary** | Bayesian networks embedded at compile time via `include_bytes!` — no runtime IO |

---

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
veilus-fingerprint = "0.1"
```

---

## 🚀 Quick Start

```rust
use veilus_fingerprint::{BrowserFamily, FingerprintGenerator, OsFamily};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Random fingerprint — one line
    let profile = FingerprintGenerator::random()?;
    println!("{}", profile.fingerprint.navigator.user_agent);
    println!("{:?}", profile.headers);

    // Constrained to Chrome on Windows
    let profile = FingerprintGenerator::new()
        .browser(BrowserFamily::Chrome)
        .os(OsFamily::Windows)
        .generate()?;

    // Deterministic — same seed → identical fingerprint
    let p1 = FingerprintGenerator::new().seeded(42).generate()?;
    let p2 = FingerprintGenerator::new().seeded(42).generate()?;
    assert_eq!(p1.fingerprint.navigator.user_agent, p2.fingerprint.navigator.user_agent);

    Ok(())
}
```

---

## 📖 API Reference

### `FingerprintGenerator` — Fluent Builder

```rust
FingerprintGenerator::new()          // unconstrained builder
    .browser(BrowserFamily::Chrome)  // constrain browser
    .os(OsFamily::Windows)           // constrain OS
    .locale("en-US")                 // constrain locale
    .seeded(42_u64)                  // deterministic mode
    .strict()                        // error on unsatisfiable constraints
    .generate()?;                    // → Result<BrowserProfile>

FingerprintGenerator::random()?;     // shorthand for new().generate()
```

### Browser & OS Constraints

```rust
// Browsers                          // Operating Systems
BrowserFamily::Chrome                OsFamily::Windows
BrowserFamily::Firefox               OsFamily::MacOs
BrowserFamily::Safari                OsFamily::Linux
BrowserFamily::Edge                  OsFamily::Android
BrowserFamily::Other(String)         OsFamily::Ios
                                     OsFamily::Other(String)
```

### `BrowserProfile` — Output Structure

```rust
pub struct BrowserProfile {
    pub id: [u8; 16],                    // unique per call (always random)
    pub generated_at: u64,               // unix timestamp
    pub dataset_version: String,         // Apify dataset version
    pub browser: BrowserInfo,            // { name, version, family }
    pub operating_system: OperatingSystem,// { name, version, family }
    pub device: DeviceType,              // Desktop | Mobile | Tablet
    pub headers: HttpHeaders,            // ordered HTTP headers (IndexMap)
    pub fingerprint: BrowserFingerprint, // full fingerprint data ↓
}
```

### `BrowserFingerprint` — Full Fingerprint Data

All fields match [`browserforge`](https://github.com/daijro/browserforge) output 1:1 with `camelCase` JSON serialization:

```rust
pub struct BrowserFingerprint {
    pub navigator: NavigatorFingerprint,       // UA, platform, vendor, languages, ...
    pub screen: ScreenFingerprint,             // resolution, DPR, avail*, outer*, ...
    pub video_card: Option<VideoCard>,         // WebGL renderer & vendor
    pub audio_codecs: Option<AudioCodecs>,     // ogg, mp3, wav, m4a, aac
    pub video_codecs: Option<VideoCodecs>,     // ogg, h264, webm
    pub battery: Option<Battery>,              // charging, level, timing
    pub fonts: Option<Vec<String>>,            // detected font families
    pub plugins_data: Option<PluginsData>,     // browser plugins & MIME types
    pub multimedia_devices: Option<MultimediaDevices>, // speakers, micros, webcams
    pub mock_web_rtc: Option<bool>,            // true=Chrome/Edge, false=Firefox/Safari
    pub slim: Option<bool>,                    // always false
}
```

### `NavigatorFingerprint` — Detailed Navigator

```rust
pub struct NavigatorFingerprint {
    pub user_agent: String,                     // navigator.userAgent
    pub hardware_concurrency: u8,               // navigator.hardwareConcurrency
    pub device_memory: Option<f32>,             // navigator.deviceMemory
    pub platform: String,                       // navigator.platform
    pub language: String,                       // navigator.language
    pub languages: Vec<String>,                 // navigator.languages
    pub webdriver: bool,                        // always false
    pub vendor: String,                         // navigator.vendor
    pub product_sub: String,                    // navigator.productSub
    pub user_agent_data: Option<UserAgentData>, // UA Client Hints (Chromium only)
    pub do_not_track: Option<String>,           // navigator.doNotTrack
    pub app_code_name: Option<String>,          // navigator.appCodeName
    pub app_name: Option<String>,               // navigator.appName
    pub app_version: Option<String>,            // navigator.appVersion
    pub max_touch_points: Option<u8>,           // navigator.maxTouchPoints
    pub extra_properties: Option<ExtraProperties>, // vendorFlavors, pdfViewerEnabled
}
```

### `UserAgentData` — High-Entropy Client Hints

Chrome and Edge profiles include full UA Client Hints data:

```rust
pub struct UserAgentData {
    pub brands: Vec<BrandVersion>,              // [{brand, version}, ...]
    pub mobile: bool,
    pub platform: String,                       // e.g., "Windows"
    pub architecture: Option<String>,           // e.g., "x86"
    pub bitness: Option<String>,                // e.g., "64"
    pub model: Option<String>,                  // device model (mobile only)
    pub platform_version: Option<String>,       // e.g., "10.0.0"
    pub ua_full_version: Option<String>,        // e.g., "125.0.6422.141"
    pub full_version_list: Option<Vec<BrandVersion>>,
}
```

---

## 📋 Full JSON Output Example

```bash
cargo run --example full_output -p veilus-fingerprint
```

<details>
<summary>Click to expand sample JSON</summary>

```json
{
  "id": [38, 14, 201, 106, 73, 133, 8, 240, 187, 103, 131, 126, 67, 211, 73, 97],
  "generatedAt": 1775385049,
  "datasetVersion": "2024-01",
  "browser": {
    "name": "chrome",
    "version": "105.0.0.0",
    "family": "chrome"
  },
  "operatingSystem": {
    "name": "windows",
    "version": "unknown",
    "family": "windows"
  },
  "device": "desktop",
  "headers": {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ...",
    "Accept": "text/html,application/xhtml+xml,...",
    "Accept-Encoding": "gzip",
    "Upgrade-Insecure-Requests": "1"
  },
  "fingerprint": {
    "navigator": {
      "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ...",
      "hardwareConcurrency": 32,
      "deviceMemory": 8.0,
      "platform": "Linux x86_64",
      "language": "en-US",
      "languages": ["en-US"],
      "webdriver": false,
      "vendor": "Google Inc.",
      "productSub": "20030107",
      "userAgentData": {
        "brands": [
          { "brand": "Google Chrome", "version": "131" },
          { "brand": "Chromium", "version": "131" },
          { "brand": "Not_A Brand", "version": "24" }
        ],
        "mobile": false,
        "platform": "Windows",
        "architecture": "x86",
        "bitness": "64",
        "platformVersion": "19.0.0",
        "uaFullVersion": "131.0.6778.267",
        "fullVersionList": [
          { "brand": "Google Chrome", "version": "131.0.6778.267" },
          { "brand": "Chromium", "version": "131.0.6778.267" },
          { "brand": "Not_A Brand", "version": "24.0.0.0" }
        ]
      },
      "appCodeName": "Mozilla",
      "appName": "Netscape",
      "appVersion": "5.0 (Windows NT 10.0; Win64; x64) ...",
      "maxTouchPoints": 0,
      "product": "Gecko",
      "extraProperties": {
        "vendorFlavors": ["chrome"],
        "installedApps": []
      }
    },
    "screen": {
      "width": 1280,
      "height": 1200,
      "availWidth": 1280,
      "availHeight": 1200,
      "colorDepth": 24,
      "pixelDepth": 24,
      "devicePixelRatio": 1.0,
      "innerWidth": 0,
      "innerHeight": 0,
      "availTop": 0,
      "availLeft": 0,
      "outerWidth": 1280,
      "outerHeight": 1200
    },
    "videoCard": {
      "renderer": "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device ...), SwiftShader driver)",
      "vendor": "Google Inc. (Google)"
    },
    "audioCodecs": {
      "ogg": "probably",
      "mp3": "probably",
      "wav": "probably",
      "m4a": "",
      "aac": ""
    },
    "videoCodecs": {
      "ogg": "",
      "h264": "",
      "webm": "probably"
    },
    "battery": {
      "charging": true,
      "chargingTime": 0.0,
      "dischargingTime": null,
      "level": 1.0
    },
    "fonts": [],
    "mockWebRtc": true,
    "slim": false
  }
}
```

</details>

---

## 🔧 Usage Patterns

### Generate HTTP Headers Only

```rust
use veilus_fingerprint::{BrowserFamily, FingerprintGenerator};

let profile = FingerprintGenerator::new()
    .browser(BrowserFamily::Chrome)
    .generate()?;

// Use with reqwest, hyper, or any HTTP client
for (key, value) in &profile.headers {
    println!("{key}: {value}");
}
// User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) ...
// Accept: text/html,application/xhtml+xml,...
// sec-ch-ua: "Google Chrome";v="125", "Chromium";v="125", ...
```

### Access Extended Fingerprint Fields

```rust
let profile = FingerprintGenerator::new().seeded(777).generate()?;
let fp = &profile.fingerprint;

// WebGL Video Card
if let Some(vc) = &fp.video_card {
    println!("GPU: {} — {}", vc.vendor, vc.renderer);
}

// Battery Status
if let Some(bat) = &fp.battery {
    println!("Battery: {:.0}% (charging: {})", bat.level * 100.0, bat.charging);
}

// UA Client Hints (Chromium only)
if let Some(uad) = &fp.navigator.user_agent_data {
    println!("Architecture: {:?}, Bitness: {:?}", uad.architecture, uad.bitness);
}

// Detected Fonts
if let Some(fonts) = &fp.fonts {
    println!("Fonts: {:?}", fonts);
}

// WebRTC Flag
println!("mockWebRTC: {:?}", fp.mock_web_rtc); // Some(true) for Chrome/Edge
```

### Session-Based Deterministic Generation

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn session_seed(id: &str) -> u64 {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

// Same session ID → identical fingerprint every time
let profile = FingerprintGenerator::new()
    .seeded(session_seed("user-alice-session-1"))
    .browser(BrowserFamily::Chrome)
    .os(OsFamily::Windows)
    .generate()?;
```

### Strict Mode & Error Handling

```rust
use veilus_fingerprint::FingerprintError;

match FingerprintGenerator::new()
    .browser(BrowserFamily::Safari)
    .os(OsFamily::Windows)
    .strict()
    .generate()
{
    Err(FingerprintError::ConstraintConflict { browser, os }) => {
        eprintln!("{browser} is not available on {os}");
    }
    Err(FingerprintError::ConstraintsTooRestrictive(msg)) => {
        eprintln!("Constraints cannot be satisfied: {msg}");
    }
    Ok(profile) => { /* use profile */ }
    Err(e) => eprintln!("Unexpected: {e}"),
}
```

| Error Variant | When |
|---------------|------|
| `ConstraintConflict` | Known impossible combo (e.g., Safari + Windows) |
| `ConstraintsTooRestrictive` | Sampler exhausted retry budget (only with `.strict()`) |
| `NetworkParseError` | Embedded Bayesian network is corrupt (should never happen) |
| `SamplingFailed` | Internal sampler error |

---

## 📚 Examples

Run any example with:

```bash
cargo run --example <name> -p veilus-fingerprint
```

| Example | Description | browserforge equivalent |
|---------|-------------|------------------------|
| [`basic`](fingerprint-rs/examples/basic.rs) | Core API usage patterns | `FingerprintGenerator().generate()` |
| [`full_output`](fingerprint-rs/examples/full_output.rs) | Complete JSON output for comparison | `json.dumps(fg.generate().__dict__)` |
| [`headers`](fingerprint-rs/examples/headers.rs) | HTTP header generation & integration | `HeaderGenerator(browser='chrome').generate()` |
| [`extended`](fingerprint-rs/examples/extended.rs) | All extended fields: GPU, codecs, battery, fonts, plugins | Full fingerprint inspection |
| [`all_browsers`](fingerprint-rs/examples/all_browsers.rs) | Side-by-side browser comparison | Multi-browser generation |
| [`seeded_batch`](fingerprint-rs/examples/seeded_batch.rs) | Session-based deterministic generation | Reproducible sessions |
| [`benchmark`](fingerprint-rs/examples/benchmark.rs) | Performance measurement | Throughput comparison |

---

## ⚡ Performance

Benchmarked on Apple M-series (release build):

| Metric | Result |
|--------|--------|
| Cold start (first call) | ~70ms (includes network decompression) |
| Warm (unconstrained) | **~127µs/call** → 7,800+ fps |
| Warm (constrained Chrome+Windows) | **~181µs/call** → 5,500+ fps |

```bash
cargo run --release --example benchmark -p veilus-fingerprint
```

> **Comparison**: browserforge (Python) reports 0.1–0.2ms per generation. veilus-fingerprint matches or exceeds this performance while running in pure Rust with zero FFI overhead.

---

## 🏗️ Architecture

### Workspace Structure

```
fingerprint-generator/               ← Cargo workspace root
├── fingerprint-core/                 ← Shared types & errors
│   └── src/types/fingerprint.rs      ← BrowserProfile, NavigatorFingerprint, etc.
├── fingerprint-data/                 ← Embedded Bayesian network ZIPs
│   ├── data/                         ← header-network.zip, fingerprint-network.zip
│   └── src/loader.rs                 ← Lazy decompression via OnceLock
└── fingerprint-rs/                   ← Public API crate
    ├── src/
    │   ├── generator.rs              ← FingerprintGenerator builder
    │   ├── assembler.rs              ← Raw network → BrowserProfile mapping
    │   └── engine/                   ← Bayesian sampler (ancestral + constrained)
    └── examples/                     ← 7 runnable examples
```

### Dual-Network Architecture

veilus-fingerprint uses **two independent Bayesian networks**:

1. **Header Network** — generates realistic HTTP headers, browser family, OS, and device type. Constraints (browser, OS) are applied here via rejection sampling.
2. **Fingerprint Network** — generates JavaScript API values (navigator, screen, WebGL, codecs, etc.). Sampled independently from the header network.

The assembler then **merges** outputs from both networks into a coherent `BrowserProfile`, deriving computed fields like `mockWebRTC` from the header network's browser family.

### Advanced: Lower-Level API

Access the Bayesian network and sampler directly for custom use cases:

```rust
use veilus_fingerprint::{sample_ancestral, sample_constrained, Constraints};
use fingerprint_data::loader::get_header_network;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

let network = get_header_network()?;
let mut rng = ChaCha8Rng::seed_from_u64(42);

// Unconstrained sample — raw node assignments
let assignment = sample_ancestral(network, &mut rng)?;

// Constrained sample — rejection sampling with target values
let mut constraints = Constraints::new();
constraints.insert("*OPERATING_SYSTEM".to_string(), vec!["windows".to_string()]);
let assignment = sample_constrained(network, &constraints, &mut rng)?;
```

---

## 🔍 browserforge Feature Parity

| Feature | browserforge (Python) | veilus-fingerprint (Rust) |
|---------|----------------------|----------------------|
| Navigator (UA, platform, vendor) | ✅ | ✅ |
| HTTP Headers (ordered) | ✅ | ✅ |
| Screen (resolution, DPR) | ✅ | ✅ |
| Screen extended (outer*, avail*, client*) | ✅ | ✅ |
| UA Client Hints (high-entropy) | ✅ | ✅ |
| WebGL VideoCard (renderer, vendor) | ✅ | ✅ |
| Audio Codecs (ogg, mp3, wav, m4a, aac) | ✅ | ✅ |
| Video Codecs (ogg, h264, webm) | ✅ | ✅ |
| Battery Status (charging, level, time) | ✅ | ✅ |
| Fonts (detected font families) | ✅ | ✅ |
| Plugins & MIME types | ✅ | ✅ |
| Multimedia Devices (speakers, micros, webcams) | ✅ | ✅ |
| mockWebRTC flag | ✅ | ✅ |
| Browser constraint | ✅ | ✅ |
| OS constraint | ✅ | ✅ |
| Device constraint (mobile/desktop) | ✅ | 🔜 |
| Seeded/deterministic generation | ❌ | ✅ |
| Strict mode (fail on impossible combos) | ❌ | ✅ |
| Playwright/Puppeteer injector | ✅ | 🔜 |

---

## 🧪 Testing

```bash
# Run the full test suite (58 tests across 3 crates)
cargo test --workspace

# Run with clippy (zero warnings enforced)
cargo clippy --workspace --all-targets -- -D warnings
```

The test suite includes:
- **Structural tests** — profile shape, field presence, JSON serialization
- **Constraint tests** — browser/OS filtering, conflict detection, strict mode
- **Extended field tests** — videoCard, codecs, battery, fonts, plugins, UAD, mockWebRTC
- **Determinism tests** — seed reproducibility, different seeds → different outputs
- **Header sanitization** — no internal meta-keys (`*BROWSER`, `*DEVICE`) leak to output

---

## 📜 Data Source

Browser fingerprint distributions are derived from empirical data collected by [Apify](https://apify.com):

- [`@apify/fingerprint-generator`](https://github.com/apify/fingerprint-suite) — fingerprint Bayesian network
- [`@apify/header-generator`](https://github.com/apify/fingerprint-suite) — header Bayesian network

The Bayesian network ZIP files are embedded at compile time via `include_bytes!` and validated by `build.rs`. No runtime file IO is required.

---

## 📄 License

[MIT](LICENSE)

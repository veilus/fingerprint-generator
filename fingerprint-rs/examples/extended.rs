//! # Extended Fingerprint Fields
//!
//! Demonstrates access to all extended fingerprint data equivalent to
//! `browserforge`'s full output: videoCard, codecs, battery, fonts,
//! plugins, multimedia devices, userAgentData high-entropy hints, etc.
//!
//! Run with:
//! ```bash
//! cargo run --example extended -p fingerprint-rs
//! ```

use veilus_fingerprint::FingerprintGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║     fingerprint-rs · Extended Fingerprint        ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    let profile = FingerprintGenerator::new()
        .seeded(777)
        .generate()?;

    let fp = &profile.fingerprint;
    let nav = &fp.navigator;

    // ── Navigator basics ──────────────────────────────────────────────────
    println!("━━━ Navigator ━━━");
    println!("  userAgent       : {}", nav.user_agent);
    println!("  platform        : {}", nav.platform);
    println!("  hardwareConcur. : {}", nav.hardware_concurrency);
    println!("  deviceMemory    : {:.0}", nav.device_memory.unwrap_or(0.0));
    println!("  vendor          : {}", nav.vendor);
    println!("  language        : {}", nav.language);
    println!("  webdriver       : {} (always false)", nav.webdriver);
    if let Some(dnt) = &nav.do_not_track {
        println!("  doNotTrack      : {dnt}");
    }
    if let Some(mtp) = nav.max_touch_points {
        println!("  maxTouchPoints  : {mtp}");
    }
    println!();

    // ── UA Client Hints (high-entropy) ────────────────────────────────────
    println!("━━━ User-Agent Client Hints ━━━");
    if let Some(uad) = &nav.user_agent_data {
        println!("  platform        : {}", uad.platform);
        println!("  mobile          : {}", uad.mobile);
        println!("  brands          :");
        for b in &uad.brands {
            println!("    - {} v{}", b.brand, b.version);
        }
        if let Some(arch) = &uad.architecture {
            println!("  architecture    : {arch}");
        }
        if let Some(bits) = &uad.bitness {
            println!("  bitness         : {bits}");
        }
        if let Some(pv) = &uad.platform_version {
            println!("  platformVersion : {pv}");
        }
        if let Some(fv) = &uad.ua_full_version {
            println!("  uaFullVersion   : {fv}");
        }
        if let Some(fvl) = &uad.full_version_list {
            println!("  fullVersionList :");
            for b in fvl {
                println!("    - {} v{}", b.brand, b.version);
            }
        }
    } else {
        println!("  (not present — non-Chromium browser)");
    }
    println!();

    // ── Screen ────────────────────────────────────────────────────────────
    println!("━━━ Screen ━━━");
    let s = &fp.screen;
    println!("  resolution      : {}×{}", s.width, s.height);
    println!("  availSize       : {}×{}", s.avail_width, s.avail_height);
    println!("  colorDepth      : {}", s.color_depth);
    println!("  pixelDepth      : {}", s.pixel_depth);
    println!("  devicePixelRatio: {:.2}", s.device_pixel_ratio);
    if let Some(ow) = s.outer_width {
        println!("  outerWidth      : {ow}");
    }
    if let Some(oh) = s.outer_height {
        println!("  outerHeight     : {oh}");
    }
    println!();

    // ── WebGL / VideoCard ─────────────────────────────────────────────────
    println!("━━━ VideoCard (WebGL) ━━━");
    if let Some(vc) = &fp.video_card {
        println!("  vendor   : {}", vc.vendor);
        println!("  renderer : {}", vc.renderer);
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Audio Codecs ──────────────────────────────────────────────────────
    println!("━━━ Audio Codecs ━━━");
    if let Some(ac) = &fp.audio_codecs {
        println!("  ogg : {}", ac.ogg);
        println!("  mp3 : {}", ac.mp3);
        println!("  wav : {}", ac.wav);
        println!("  m4a : {}", ac.m4a);
        println!("  aac : {}", ac.aac);
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Video Codecs ──────────────────────────────────────────────────────
    println!("━━━ Video Codecs ━━━");
    if let Some(vc) = &fp.video_codecs {
        println!("  ogg  : {}", vc.ogg);
        println!("  h264 : {}", vc.h264);
        println!("  webm : {}", vc.webm);
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Battery ───────────────────────────────────────────────────────────
    println!("━━━ Battery ━━━");
    if let Some(bat) = &fp.battery {
        println!("  charging        : {}", bat.charging);
        println!("  chargingTime    : {:?}", bat.charging_time);
        println!("  dischargingTime : {:?}", bat.discharging_time);
        println!("  level           : {:.0}%", bat.level * 100.0);
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Fonts ─────────────────────────────────────────────────────────────
    println!("━━━ Fonts ━━━");
    if let Some(fonts) = &fp.fonts {
        println!("  {} fonts detected:", fonts.len());
        for (i, font) in fonts.iter().enumerate().take(10) {
            println!("    {}: {font}", i + 1);
        }
        if fonts.len() > 10 {
            println!("    … and {} more", fonts.len() - 10);
        }
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Plugins ───────────────────────────────────────────────────────────
    println!("━━━ Plugins ━━━");
    if let Some(pd) = &fp.plugins_data {
        println!("  {} plugins:", pd.plugins.len());
        for plugin in &pd.plugins {
            println!("    - {} ({})", plugin.name, plugin.filename);
        }
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Multimedia Devices ────────────────────────────────────────────────
    println!("━━━ Multimedia Devices ━━━");
    if let Some(mm) = &fp.multimedia_devices {
        println!("  speakers : {}", mm.speakers);
        println!("  micros   : {}", mm.micros);
        println!("  webcams  : {}", mm.webcams);
    } else {
        println!("  (not present)");
    }
    println!();

    // ── Flags ─────────────────────────────────────────────────────────────
    println!("━━━ Flags ━━━");
    println!("  mockWebRTC : {:?}", fp.mock_web_rtc);
    println!("  slim       : {:?}", fp.slim);

    // ── Extra Properties ──────────────────────────────────────────────────
    if let Some(extra) = &nav.extra_properties {
        println!();
        println!("━━━ Extra Properties ━━━");
        if let Some(flavors) = &extra.vendor_flavors {
            println!("  vendorFlavors    : {:?}", flavors);
        }
        if let Some(pdf) = extra.pdf_viewer_enabled {
            println!("  pdfViewerEnabled : {pdf}");
        }
        if let Some(apps) = &extra.installed_apps {
            println!("  installedApps    : {:?}", apps);
        }
    }

    Ok(())
}

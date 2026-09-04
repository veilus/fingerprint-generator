use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{Rng, RngCore};
use veilus_fingerprint_core::{
    AudioCodecs, Battery, BrandVersion, BrowserFamily, BrowserFingerprint, BrowserInfo,
    BrowserProfile, DeviceType, ExtraProperties, FingerprintError, HttpHeaders, MultimediaDevices,
    NavigatorFingerprint, OperatingSystem, OsFamily, PluginsData, ScreenFingerprint, UserAgentData,
    VideoCard, VideoCodecs,
};
use veilus_fingerprint_data::network::STRINGIFIED_PREFIX;
use veilus_fingerprint_data::DATASET_VERSION;

// ── Parse helpers ──────────────────────────────────────────────────────────

/// Parse a `*STRINGIFIED*{...}` value by stripping the prefix and deserializing.
fn parse_stringified<T: serde::de::DeserializeOwned>(raw: &str) -> Option<T> {
    let json = raw.strip_prefix(STRINGIFIED_PREFIX)?;
    serde_json::from_str(json).ok()
}

/// Get optional field, stripping `*MISSING_VALUE*` sentinel.
fn opt_field(map: &HashMap<String, String>, key: &str) -> Option<String> {
    map.get(key)
        .filter(|v| v.as_str() != veilus_fingerprint_data::network::MISSING_VALUE)
        .cloned()
}

/// Parse a u8 from a `*STRINGIFIED*N` value.
fn parse_stringified_u8(raw: &str) -> Option<u8> {
    parse_stringified::<u8>(raw).or_else(|| raw.parse().ok())
}

/// Parse an f32 from a `*STRINGIFIED*N` value.
fn parse_stringified_f32(raw: &str) -> Option<f32> {
    parse_stringified::<f32>(raw).or_else(|| raw.parse().ok())
}

/// Parse BrowserFamily from `*BROWSER` network value like `"chrome/120.0.0.0"`.
fn parse_browser_family(browser_str: &str) -> (BrowserFamily, String) {
    let (name, version) = if let Some(pos) = browser_str.find('/') {
        (&browser_str[..pos], browser_str[pos + 1..].to_string())
    } else {
        (browser_str, String::from("unknown"))
    };

    let family = match name.to_lowercase().as_str() {
        "chrome" => BrowserFamily::Chrome,
        "firefox" => BrowserFamily::Firefox,
        "safari" => BrowserFamily::Safari,
        "edge" => BrowserFamily::Edge,
        other => BrowserFamily::Other(other.to_string()),
    };
    (family, version)
}

/// Sua DINH DANG cua `platformVersion`, khong sua GIA TRI.
///
/// Chrome gui `major.minor.patch` ngan bang dau CHAM. Du lieu Apify chua hai
/// dang sai khong ban cai duoc, do 2026-09-04:
///
/// ```text
/// "10_15_7"  ->  "10.15.7"   dau gach duoi la dang cua chuoi UA, khong phai UA-CH
/// "10.0"     ->  "10.0.0"    thieu phan thu ba
/// ```
///
/// CHI sua hai dang do. KHONG dung o day: chuoi rong, va cac gia tri ma luat
/// cua ta cho la sai nhung chua chac ta dung — vi du Linux gui phien ban
/// kernel ("6.8.0", 25/500 ho so). Dat mot gia tri vao cho rong la BIA, va
/// sua mot gia tri vi luat cua ta noi vay thi phai chac luat do dung truoc.
///
/// Tra `None` khi khong co gi de sua.
fn platform_version_chuan_hoa(pv: &str) -> Option<String> {
    if pv.is_empty() {
        return None;
    }
    let cham = pv.replace('_', ".");
    let mut phan: Vec<&str> = cham.split('.').collect();
    if phan.iter().any(|x| x.parse::<u32>().is_err()) {
        return None;
    }
    while phan.len() < 3 {
        phan.push("0");
    }
    phan.truncate(3);
    let ket = phan.join(".");
    (ket != pv).then_some(ket)
}

/// Kien truc CPU suy tu chuoi renderer WebGL.
///
/// Chrome gui "arm" tren Apple Silicon va tren ARM64 noi chung, "x86" con lai.
/// Day la mot phep suy DINH NGHIA, khong phai phong doan: renderer noi may
/// chay chip gi, va Chrome khong co lua chon nao khac.
fn kien_truc_tu_renderer(r: &str) -> &'static str {
    let arm = r.contains("Apple M")
        || r.contains("ANGLE (Apple")
        || r.contains("aarch64")
        || r.contains("arm64")
        || r.contains("Mali")
        || r.contains("Adreno");
    if arm {
        "arm"
    } else {
        "x86"
    }
}

/// Parse OsFamily from `*OPERATING_SYSTEM` network value like `"windows"`.
fn parse_os_family(os_str: &str) -> OsFamily {
    match os_str.to_lowercase().as_str() {
        "windows" => OsFamily::Windows,
        "macos" => OsFamily::MacOs,
        "linux" => OsFamily::Linux,
        "android" => OsFamily::Android,
        "ios" => OsFamily::Ios,
        other => OsFamily::Other(other.to_string()),
    }
}

/// Parse DeviceType from `*DEVICE` network value.
fn parse_device_type(device_str: &str) -> DeviceType {
    match device_str.to_lowercase().as_str() {
        "mobile" => DeviceType::Mobile,
        "tablet" => DeviceType::Tablet,
        _ => DeviceType::Desktop,
    }
}

// ── Assemble ────────────────────────────────────────────────────────────

/// Assemble a `BrowserProfile` from raw Bayesian network samples.
///
/// # Arguments
///
/// * `fp` — Assignment map from the fingerprint Bayesian network.
/// * `headers` — Assignment map from the header Bayesian network.
/// * `rng` — Seeded RNG for reproducible fingerprint fields.
///
/// The profile `id` and `generated_at` are always random/current-time regardless of seed.
pub fn assemble_profile(
    fp: &HashMap<String, String>,
    headers: &HashMap<String, String>,
    rng: &mut impl Rng,
) -> Result<BrowserProfile, FingerprintError> {
    // Profile ID: always random (not seeded) — unique per generation
    let mut id = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut id);

    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // ── Browser & OS from header network ────────────────────────────────────
    let browser_raw = headers.get("*BROWSER").cloned().unwrap_or_default();
    let (browser_family, browser_version) = parse_browser_family(&browser_raw);

    let os_raw = headers
        .get("*OPERATING_SYSTEM")
        .cloned()
        .unwrap_or_default();
    let os_family = parse_os_family(&os_raw);

    let device_raw = headers.get("*DEVICE").cloned().unwrap_or_default();
    let device_type = parse_device_type(&device_raw);

    let browser_name = browser_raw
        .find('/')
        .map(|i| browser_raw[..i].to_string())
        .unwrap_or_else(|| browser_raw.clone());

    let os_name = os_raw.clone();

    // ── Navigator from fingerprint network ──────────────────────────────────
    let user_agent = fp.get("userAgent").cloned().unwrap_or_else(|| {
        format!(
            "Mozilla/5.0 ({}) {}/{}",
            os_name, browser_name, browser_version
        )
    });

    let hardware_concurrency = fp
        .get("hardwareConcurrency")
        .and_then(|v| parse_stringified_u8(v))
        .unwrap_or(4);

    let device_memory = fp
        .get("deviceMemory")
        .and_then(|v| parse_stringified_f32(v));

    let platform = opt_field(fp, "platform").unwrap_or_else(|| match &os_family {
        OsFamily::Windows => "Win32".to_string(),
        OsFamily::MacOs => "MacIntel".to_string(),
        OsFamily::Linux => "Linux x86_64".to_string(),
        _ => String::new(),
    });

    let vendor = opt_field(fp, "vendor").unwrap_or_else(|| match &browser_family {
        BrowserFamily::Chrome | BrowserFamily::Edge => "Google Inc.".to_string(),
        BrowserFamily::Safari => "Apple Computer, Inc.".to_string(),
        _ => String::new(),
    });

    let product_sub = opt_field(fp, "productSub").unwrap_or_else(|| "20030107".to_string());

    let language = "en-US".to_string();
    let languages = vec!["en-US".to_string()];

    // ── userAgentData from fingerprint network `userAgentData` node ──────────
    // This is a *STRINGIFIED* JSON with the full structure including high-entropy fields
    let user_agent_data: Option<UserAgentData> = fp
        .get("userAgentData")
        .and_then(|v| {
            // Try to parse the full structure from the network
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RawUad {
                brands: Option<Vec<BrandVersion>>,
                mobile: Option<bool>,
                platform: Option<String>,
                architecture: Option<String>,
                bitness: Option<String>,
                model: Option<String>,
                platform_version: Option<String>,
                ua_full_version: Option<String>,
                full_version_list: Option<Vec<BrandVersion>>,
            }
            let raw: RawUad = parse_stringified(v)?;
            let brands = raw.brands.unwrap_or_default();
            // Discard empty UAD — Firefox/Safari return brands=[] from network
            if brands.is_empty() {
                return None;
            }
            Some(UserAgentData {
                brands,
                mobile: raw.mobile.unwrap_or(false),
                platform: raw.platform.unwrap_or_default(),
                architecture: raw.architecture,
                bitness: raw.bitness,
                model: raw.model,
                platform_version: raw.platform_version,
                ua_full_version: raw.ua_full_version,
                full_version_list: raw.full_version_list,
            })
        })
        .or_else(|| {
            // Fallback: construct from browser info (for Chrome/Edge only)
            match &browser_family {
                BrowserFamily::Chrome | BrowserFamily::Edge => {
                    let (major_version, _) = browser_version
                        .split_once('.')
                        .unwrap_or((&browser_version, ""));
                    Some(UserAgentData {
                        brands: vec![
                            BrandVersion {
                                brand: match &browser_family {
                                    BrowserFamily::Chrome => "Google Chrome".to_string(),
                                    _ => "Microsoft Edge".to_string(),
                                },
                                version: major_version.to_string(),
                            },
                            BrandVersion {
                                brand: "Chromium".to_string(),
                                version: major_version.to_string(),
                            },
                        ],
                        mobile: matches!(device_type, DeviceType::Mobile),
                        platform: match &os_family {
                            OsFamily::Windows => "Windows".to_string(),
                            OsFamily::MacOs => "macOS".to_string(),
                            OsFamily::Linux => "Linux".to_string(),
                            OsFamily::Android => "Android".to_string(),
                            OsFamily::Ios => "iOS".to_string(),
                            OsFamily::Other(s) => s.clone(),
                        },
                        architecture: None,
                        bitness: None,
                        model: None,
                        platform_version: None,
                        ua_full_version: None,
                        full_version_list: None,
                    })
                }
                _ => None,
            }
        });

    // ── Additional navigator fields from fingerprint network ────────────────
    let do_not_track = opt_field(fp, "doNotTrack");
    let app_code_name = opt_field(fp, "appCodeName").or_else(|| Some("Mozilla".to_string()));
    let app_name = opt_field(fp, "appName").or_else(|| Some("Netscape".to_string()));
    let app_version = opt_field(fp, "appVersion");
    let oscpu = opt_field(fp, "oscpu");
    let vendor_sub = opt_field(fp, "vendorSub");
    let product = opt_field(fp, "product").or_else(|| Some("Gecko".to_string()));
    let max_touch_points = fp
        .get("maxTouchPoints")
        .and_then(|v| parse_stringified_u8(v));

    // extraProperties: *STRINGIFIED*{...}
    let extra_properties: Option<ExtraProperties> =
        fp.get("extraProperties").and_then(|v| parse_stringified(v));

    // Renderer doc SOM, chi de kiem tra nhat quan UA-CH ngay duoi.
    // `video_card` day du duoc dung o phia sau; cho nay chi can chuoi.
    let renderer_tho: Option<String> = fp.get("videoCard").and_then(|v| {
        #[derive(serde::Deserialize)]
        struct R {
            renderer: Option<String>,
        }
        parse_stringified::<R>(v).and_then(|r| r.renderer)
    });

    // ── UA-CH: architecture PHAI theo GPU, khong theo du lieu ────────────────
    //
    // `architecture` la HAM THUAN cua renderer: Chrome tren Apple Silicon luon
    // gui "arm", tren x86 luon gui "x86". Nen suy no ra khong phai bia — no la
    // dinh nghia.
    //
    // Vi sao can: du lieu Apify chua ca gia tri mau thuan. Do 2026-09-04 tren
    // 600 ho so Windows (GPU khong phai Apple): 3 khoi khai "arm", 3 khai
    // "x64" — "x64" con khong phai gia tri UA-CH hop le. Do la traffic that
    // cua may dang spoof hong.
    //
    // CHI ghi de khi da co khai bao va no MAU THUAN. `None` nghia la "khong
    // khai", va khong khai thi khong co gi mau thuan — dat mot gia tri vao do
    // se doi hinh dang ho so ma khong sua loi nao.
    let user_agent_data = user_agent_data.map(|mut u| {
        if let (Some(hien), Some(r)) = (u.architecture.as_deref(), renderer_tho.as_deref()) {
            let dung = kien_truc_tu_renderer(r);
            if hien != dung {
                u.architecture = Some(dung.to_string());
            }
        }
        if let Some(pv) = u.platform_version.as_deref() {
            if let Some(chuan) = platform_version_chuan_hoa(pv) {
                u.platform_version = Some(chuan);
            }
        }
        u
    });

    let navigator = NavigatorFingerprint {
        user_agent,
        hardware_concurrency,
        device_memory,
        platform,
        language,
        languages,
        webdriver: false, // ALWAYS false — never sampled
        vendor,
        product_sub,
        user_agent_data,
        do_not_track,
        app_code_name,
        app_name,
        app_version,
        oscpu,
        vendor_sub,
        max_touch_points,
        product,
        extra_properties,
    };

    // ── Screen from fingerprint network `screen` node ───────────────────────
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ScreenData {
        width: Option<u32>,
        height: Option<u32>,
        avail_width: Option<u32>,
        avail_height: Option<u32>,
        color_depth: Option<u32>,
        pixel_depth: Option<u32>,
        device_pixel_ratio: Option<f64>,
        inner_width: Option<u32>,
        inner_height: Option<u32>,
        // Extended fields
        avail_top: Option<u32>,
        avail_left: Option<u32>,
        outer_width: Option<u32>,
        outer_height: Option<u32>,
        screen_x: Option<i32>,
        page_x_offset: Option<u32>,
        page_y_offset: Option<u32>,
        client_width: Option<u32>,
        client_height: Option<u32>,
        has_hdr: Option<bool>,
    }

    let screen_data: Option<ScreenData> = fp.get("screen").and_then(|v| parse_stringified(v));

    let _ = rng; // RNG used for future screen-jitter; reserved.

    let screen = ScreenFingerprint {
        width: screen_data.as_ref().and_then(|s| s.width).unwrap_or(1920),
        height: screen_data.as_ref().and_then(|s| s.height).unwrap_or(1080),
        avail_width: screen_data
            .as_ref()
            .and_then(|s| s.avail_width)
            .unwrap_or(1920),
        avail_height: screen_data
            .as_ref()
            .and_then(|s| s.avail_height)
            .unwrap_or(1040),
        color_depth: screen_data
            .as_ref()
            .and_then(|s| s.color_depth)
            .map(|v| v as u8)
            .unwrap_or(24u8),
        pixel_depth: screen_data
            .as_ref()
            .and_then(|s| s.pixel_depth)
            .map(|v| v as u8)
            .unwrap_or(24u8),
        device_pixel_ratio: screen_data
            .as_ref()
            .and_then(|s| s.device_pixel_ratio)
            .map(|v| v as f32)
            .unwrap_or(1.0f32),
        inner_width: screen_data
            .as_ref()
            .and_then(|s| s.inner_width)
            .unwrap_or(0),
        inner_height: screen_data
            .as_ref()
            .and_then(|s| s.inner_height)
            .unwrap_or(0),
        // Extended screen fields
        avail_top: screen_data.as_ref().and_then(|s| s.avail_top),
        avail_left: screen_data.as_ref().and_then(|s| s.avail_left),
        outer_width: screen_data.as_ref().and_then(|s| s.outer_width),
        outer_height: screen_data.as_ref().and_then(|s| s.outer_height),
        screen_x: screen_data.as_ref().and_then(|s| s.screen_x),
        page_x_offset: screen_data.as_ref().and_then(|s| s.page_x_offset),
        page_y_offset: screen_data.as_ref().and_then(|s| s.page_y_offset),
        client_width: screen_data.as_ref().and_then(|s| s.client_width),
        client_height: screen_data.as_ref().and_then(|s| s.client_height),
        has_hdr: screen_data.as_ref().and_then(|s| s.has_hdr),
    };

    // ── Extended fingerprint data from fingerprint network ───────────────────
    let video_card: Option<VideoCard> = fp.get("videoCard").and_then(|v| parse_stringified(v));

    let audio_codecs: Option<AudioCodecs> =
        fp.get("audioCodecs").and_then(|v| parse_stringified(v));

    let video_codecs: Option<VideoCodecs> =
        fp.get("videoCodecs").and_then(|v| parse_stringified(v));

    let battery: Option<Battery> = fp.get("battery").and_then(|v| parse_stringified(v));

    let multimedia_devices: Option<MultimediaDevices> = fp
        .get("multimediaDevices")
        .and_then(|v| parse_stringified(v));

    let plugins_data: Option<PluginsData> =
        fp.get("pluginsData").and_then(|v| parse_stringified(v));

    let fonts: Option<Vec<String>> = fp.get("fonts").and_then(|v| parse_stringified(v));

    // ── HTTP Headers from header network ────────────────────────────────────
    // BUG FIX: Filter out ALL `*` prefixed keys (internal Bayesian network metadata)
    let mut http_headers = HttpHeaders::new();
    for (key, value) in headers {
        // Skip internal `*PREFIX` nodes (e.g., *BROWSER, *OPERATING_SYSTEM, *DEVICE, *HTTP_VERSION)
        if key.starts_with('*') {
            continue;
        }
        // Skip MISSING_VALUE sentinels
        if value == veilus_fingerprint_data::network::MISSING_VALUE {
            continue;
        }
        http_headers.insert(key.clone(), value.clone());
    }

    // mockWebRTC: true for browsers that expose WebRTC APIs by default
    let mock_web_rtc = matches!(browser_family, BrowserFamily::Chrome | BrowserFamily::Edge);

    Ok(BrowserProfile {
        id,
        generated_at,
        dataset_version: DATASET_VERSION.to_string(),
        browser: BrowserInfo {
            name: browser_name,
            version: browser_version,
            family: browser_family,
        },
        operating_system: OperatingSystem {
            name: os_name,
            version: String::from("unknown"),
            family: os_family,
        },
        device: device_type,
        headers: http_headers,
        fingerprint: BrowserFingerprint {
            navigator,
            screen,
            video_card,
            audio_codecs,
            video_codecs,
            battery,
            multimedia_devices,
            plugins_data,
            fonts,
            mock_web_rtc: Some(mock_web_rtc),
            slim: Some(false),
        },
    })
}

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

/// Tran so nhan cho ho so macOS. 24 = Mac Pro M2 Ultra, may nhieu nhan nhat
/// Apple ban. Mot con so TRICH DUOC, khong phai mot nguong bia — do la dieu
/// kien de luat loc nay khong thanh mot lua chon tuy tien. Xem VEIL-738.
pub(crate) const MAC_MAX_CORES: u16 = 24;

/// Tran cho iOS. 10 = iPad Pro M4 (CPU 10 nhan, thong so cua Apple); iPhone 16
/// Pro co 6. Cung la con so trich duoc.
pub(crate) const IOS_MAX_CORES: u16 = 10;

/// Tran cho Android. KHAC HAI CAI TREN: day KHONG phai so nhan cua mot may cu
/// the, ma la mot TRAN CO BIEN — khong co mot "may Android nhieu nhan nhat"
/// duy nhat de trich. SoC dau bang hien ban co 8 nhan; 16 la tran rong rai nam
/// tren moi thiet bi dang luu hanh. Neu mai co may vuot, sua o day.
pub(crate) const ANDROID_MAX_CORES: u16 = 16;

/// Cat so nhan xuong nguong kha di cua he dieu hanh ma ho so khai.
///
/// CHI macOS co tran. Linux va Windows khong, vi server that dat toi 384 luong
/// (EPYC 9754 hai socket = 256 nhan / 512 luong) — do 2026-09-19, gia tri `384`
/// di kem UA Linux voi khoi luong 72,00 va UA Windows 8,00. Cat chung la vut
/// du lieu hop le.
///
/// Con `384` di kem UA macOS voi khoi luong 46,24, va do la du lieu KHONG THE
/// dung: mot to hop khong ton tai tren doi la mot dau van tay MOI, te hon mot
/// dau van tay sai.
///
/// GOC CAT CO Y, va no co tran: moi ho so macOS vuot 24 deu ve DUNG 24, nen
/// gia tri 24 bi don cao hon tu nhien. Do la mot dau vet, chi nho hon dau vet
/// cu (moi thu ve 4). Nang cap khi co ly do that: rut lai tu phan phoi co dieu
/// kien cua chinh nut do, bo cac gia tri bat kha, thay vi cat cung.
fn clamp_cores_for_os(cores: u16, os: &OsFamily) -> u16 {
    match os {
        OsFamily::MacOs => cores.min(MAC_MAX_CORES),
        OsFamily::Ios => cores.min(IOS_MAX_CORES),
        OsFamily::Android => cores.min(ANDROID_MAX_CORES),
        // Windows va Linux KHONG co tran: server that dat toi hang tram nhan,
        // va UA khong phan biet duoc may chu voi may ban. Do 2026-09-19 tren
        // 3000 ho so: Linux max 144, Windows max 640 — ca hai deu ton tai.
        OsFamily::Windows | OsFamily::Linux | OsFamily::Other(_) => cores,
    }
}

/// Parse gia tri cua nut `hardwareConcurrency`.
fn parse_stringified_hardware_concurrency(raw: &str) -> Option<u16> {
    parse_stringified::<u16>(raw).or_else(|| raw.parse().ok())
}

/// Parse gia tri cua nut `maxTouchPoints`.
///
/// Tach khoi `parse_stringified_u8` co y: hai nut khac nhau co mien gia tri khac
/// nhau, va gop chung mot ham nghia la doi mien cua nut nay se lang le doi mien
/// cua nut kia. Xem VEIL-699.
fn parse_stringified_max_touch_points(raw: &str) -> Option<u16> {
    parse_stringified::<u16>(raw).or_else(|| raw.parse().ok())
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
        .and_then(|v| parse_stringified_hardware_concurrency(v))
        .map(|n| clamp_cores_for_os(n, &os_family))
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
        .and_then(|v| parse_stringified_max_touch_points(v));

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Lay `possibleValues` cua mot nut trong mang fingerprint.
    fn possible_values_of(node_name: &str) -> Vec<String> {
        let network = veilus_fingerprint_data::loader::get_fingerprint_network()
            .expect("mang fingerprint phai nap duoc");
        let node = network
            .nodes
            .iter()
            .find(|n| n.name == node_name)
            .unwrap_or_else(|| panic!("mang phai co nut `{node_name}`"));
        node.possible_values.clone()
    }

    /// Nhu bai tren, cho `hardwareConcurrency` — VEIL-738.
    ///
    /// Nut nay nang hon `maxTouchPoints` 150 lan: bon gia tri vuot `u8`
    /// (384, 448, 512, 640) chiem 26,36% khoi luong xac suat toan mang, va
    /// 127/479 phan phoi co dieu kien mat hon mot nua khoi luong.
    ///
    /// Va no mat THEO KIEU TE HON: `.unwrap_or(4)` bien gia tri that thanh mot
    /// con so TRONG HOP LY, chu khong thanh `None`. Khong ai phan biet duoc
    /// "may 4 nhan" voi "gia tri that bi nuot".
    #[test]
    fn moi_gia_tri_hardwareconcurrency_trong_mang_deu_doc_lai_duoc() {
        let values = possible_values_of("hardwareConcurrency");

        assert!(
            values.len() >= 20,
            "nut hardwareConcurrency chi co {} gia tri — qua it de bai nay co nghia",
            values.len()
        );

        let lost: Vec<&String> = values
            .iter()
            .filter(|v| v.as_str() != veilus_fingerprint_data::network::MISSING_VALUE)
            .filter(|v| parse_stringified_hardware_concurrency(v).is_none())
            .collect();

        assert!(
            lost.is_empty(),
            "mang khai {} gia tri cho hardwareConcurrency nhung {} gia tri khong doc \
             lai duoc: {:?}",
            values.len(),
            lost.len(),
            lost
        );
    }

    /// Ho so macOS khong duoc mang so nhan ma khong may Mac nao co.
    ///
    /// BAI NAY CHI CO NGHIA SAU KHI KIEU DA NOI LEN u16. Truoc do, `384` roi
    /// vao `.unwrap_or(4)` nen khong ho so nao vuot tran — bai se XANH RONG,
    /// canh mot dieu khong the xay ra. Do la ly do thu tu la: noi kieu truoc,
    /// xem bai nay DO, roi moi loc.
    ///
    /// Tran 24 = Mac Pro M2 Ultra, may nhieu nhan nhat Apple ban. Do 2026-09-19,
    /// `384` di kem UA macOS voi khoi luong 46,24 — tuc no CO THAT trong du lieu
    /// va se duoc rut ra neu khong ai chan.
    #[test]
    fn he_di_dong_khong_bao_gio_khai_qua_tran_so_nhan_cua_no() {
        let network = veilus_fingerprint_data::loader::get_fingerprint_network()
            .expect("mang fingerprint phai nap duoc");
        let node = network
            .nodes
            .iter()
            .find(|n| n.name == "hardwareConcurrency")
            .expect("mang phai co nut hardwareConcurrency");

        // Hang doi chung: neu mang khong con gia tri nao > 24 thi bai duoi
        // khong hoi gi ca, va cho chet phai la O DAY.
        let tren_tran: Vec<u16> = node
            .possible_values
            .iter()
            .filter_map(|v| parse_stringified_hardware_concurrency(v))
            .filter(|n| *n > MAC_MAX_CORES)
            .collect();
        assert!(
            !tren_tran.is_empty(),
            "mang khong con gia tri nao > {MAC_MAX_CORES} — bai nay thanh vo nghia"
        );

        for (os, tran) in [
            (OsFamily::MacOs, MAC_MAX_CORES),
            (OsFamily::Ios, IOS_MAX_CORES),
            (OsFamily::Android, ANDROID_MAX_CORES),
        ] {
            for gia_tri in &tren_tran {
                let sau_loc = clamp_cores_for_os(*gia_tri, &os);
                assert!(
                    sau_loc <= tran,
                    "ho so {os:?} nhan {gia_tri} nhan sau khi loc con {sau_loc} — vuot tran \
                     {tran}. Mot to hop khong ton tai tren doi la mot dau van tay MOI."
                );
            }
        }
    }

    /// DAU-CUOI: sinh ho so that va kiem tran, thay vi goi thang ham loc.
    ///
    /// Bai nay bat mot thu ba bai kia KHONG bat duoc: ham loc dung nhung
    /// KHONG DUOC NOI vao duong sinh. Do 2026-09-19, ban dau tien cua ban sua
    /// nay chi loc macOS va bai don vi xanh het — chinh phep do dau-cuoi moi
    /// lo ra iOS con nhan 640 nhan va Android 512.
    #[test]
    fn ho_so_sinh_that_khong_he_vuot_tran_cua_he_di_dong() {
        use crate::FingerprintGenerator;

        let mut seen: std::collections::BTreeMap<String, (u16, usize)> =
            std::collections::BTreeMap::new();
        for seed in 0..300u64 {
            let p = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("sinh ho so phai chay");
            let os = p.operating_system.family.clone();
            let cores = p.fingerprint.navigator.hardware_concurrency;
            let e = seen.entry(format!("{os:?}")).or_insert((0, 0));
            e.0 = e.0.max(cores);
            e.1 += 1;

            let tran = match os {
                OsFamily::MacOs => Some(MAC_MAX_CORES),
                OsFamily::Ios => Some(IOS_MAX_CORES),
                OsFamily::Android => Some(ANDROID_MAX_CORES),
                _ => None,
            };
            if let Some(t) = tran {
                assert!(
                    cores <= t,
                    "seed {seed}: ho so {os:?} sinh ra {cores} nhan, vuot tran {t}"
                );
            }
        }

        // Hang doi chung: neu lo sinh khong he co ho so di dong nao thi khang
        // dinh o tren chua bao gio chay, va bai nay xanh rong.
        let di_dong: usize = seen
            .iter()
            .filter(|(k, _)| {
                k.as_str() == "MacOs" || k.as_str() == "Ios" || k.as_str() == "Android"
            })
            .map(|(_, (_, n))| *n)
            .sum();
        assert!(
            di_dong >= 20,
            "chi {di_dong} ho so macOS/iOS/Android trong 300 seed — qua it de bai nay co nghia; \
             phan bo: {seen:?}"
        );
    }

    /// Chieu nguoc: Linux va Windows KHONG bi cat, vi server that dat toi do.
    ///
    /// Thieu bai nay thi mot ban loc "cat het moi thu > 24" cung xanh, va no se
    /// vut 80 don vi khoi luong hop le cua Linux/Windows.
    #[test]
    fn ho_so_linux_va_windows_giu_nguyen_so_nhan_lon() {
        for os in [OsFamily::Linux, OsFamily::Windows] {
            assert_eq!(
                clamp_cores_for_os(384, &os),
                384,
                "{os:?} phai giu 384 — server that dat toi do (EPYC hai socket)"
            );
        }
    }

    /// Moi gia tri mang TU KHAI la co the xay ra thi phai doc lai duoc.
    ///
    /// Bai nay PHAI DO truoc khi doi `Option<u8>` thanh `Option<u16>` (VEIL-699).
    /// Neu no xanh ngay tu dau thi no khong canh gi — no chi dang noi rang u16
    /// chua duoc nhung so nho, mot dieu khong ai nghi ngo.
    ///
    /// Do 2026-09-19: `possibleValues` cua nut nay la
    /// `[0, 1, 2, 3, 5, 9, 10, 20, 40, 256]`, va `256` khong lot vao `u8`.
    /// Mat im lang thanh `None`, tuc nguoi dung thu vien doc ra "mang khong co
    /// du lieu cho ho so nay" trong khi mang CO.
    #[test]
    fn moi_gia_tri_maxtouchpoints_trong_mang_deu_doc_lai_duoc() {
        let values = possible_values_of("maxTouchPoints");

        // Hang doi chung: nut rong hoac mot phan tu thi bai duoi khong hoi gi.
        // Neu ai do thu gon du lieu, cho chet phai la O DAY.
        assert!(
            values.len() >= 5,
            "nut maxTouchPoints chi co {} gia tri — qua it de bai duoi co nghia",
            values.len()
        );

        let lost: Vec<&String> = values
            .iter()
            .filter(|v| v.as_str() != veilus_fingerprint_data::network::MISSING_VALUE)
            .filter(|v| parse_stringified_max_touch_points(v).is_none())
            .collect();

        assert!(
            lost.is_empty(),
            "mang khai {} gia tri cho maxTouchPoints nhung {} gia tri khong doc lai duoc: {:?}\n\
             Day la du lieu cua chinh thu vien, khong phai dau vao la — mat o day la mat im lang.",
            values.len(),
            lost.len(),
            lost
        );
    }
}

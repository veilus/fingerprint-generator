use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::types::fingerprint::BrowserFingerprint;

/// HTTP headers as an ordered map preserving HTTP/2 header order.
pub type HttpHeaders = IndexMap<String, String>;

/// Browser family classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserFamily {
    /// Google Chrome.
    Chrome,
    /// Mozilla Firefox.
    Firefox,
    /// Apple Safari.
    Safari,
    /// Microsoft Edge.
    Edge,
    /// Any other browser (preserved as-is).
    #[serde(untagged)]
    Other(String),
}

/// OS family classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OsFamily {
    /// Microsoft Windows.
    Windows,
    /// Apple macOS.
    MacOs,
    /// Linux (any distribution).
    Linux,
    /// Google Android.
    Android,
    /// Apple iOS.
    Ios,
    /// Any other OS.
    #[serde(untagged)]
    Other(String),
}

/// Device category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceType {
    /// Desktop or laptop computer.
    Desktop,
    /// Mobile phone.
    Mobile,
    /// Tablet device.
    Tablet,
}

/// Browser name, version, and family.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfo {
    /// Human-readable browser name (e.g., "Chrome").
    pub name: String,
    /// Full version string (e.g., "120.0.6099.109").
    pub version: String,
    /// Browser family classification.
    pub family: BrowserFamily,
}

/// Operating system information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatingSystem {
    /// Human-readable OS name (e.g., "Windows").
    pub name: String,
    /// OS version string (e.g., "10").
    pub version: String,
    /// OS family classification.
    pub family: OsFamily,
}

/// Top-level fingerprint output — the complete browser identity profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    /// Unique ID for this generation (random 16 bytes — NOT seeded).
    pub id: [u8; 16],
    /// Unix timestamp (seconds) at generation time.
    pub generated_at: u64,
    /// Version string of the embedded Apify dataset.
    pub dataset_version: String,
    /// Browser identity information.
    pub browser: BrowserInfo,
    /// Operating system information.
    pub operating_system: OperatingSystem,
    /// Device category.
    pub device: DeviceType,
    /// HTTP headers in network order.
    pub headers: HttpHeaders,
    /// Browser fingerprint (navigator + screen).
    pub fingerprint: BrowserFingerprint,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::fingerprint::{NavigatorFingerprint, ScreenFingerprint};

    fn make_test_profile() -> BrowserProfile {
        BrowserProfile {
            id: [0u8; 16],
            generated_at: 1_700_000_000,
            dataset_version: "2024-01".to_string(),
            browser: BrowserInfo {
                name: "Chrome".to_string(),
                version: "120.0".to_string(),
                family: BrowserFamily::Chrome,
            },
            operating_system: OperatingSystem {
                name: "Windows".to_string(),
                version: "10".to_string(),
                family: OsFamily::Windows,
            },
            device: DeviceType::Desktop,
            headers: HttpHeaders::new(),
            fingerprint: BrowserFingerprint {
                navigator: NavigatorFingerprint {
                    user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                        .to_string(),
                    hardware_concurrency: 8,
                    device_memory: Some(8.0),
                    platform: "Win32".to_string(),
                    language: "en-US".to_string(),
                    languages: vec!["en-US".to_string(), "en".to_string()],
                    webdriver: false,
                    vendor: "Google Inc.".to_string(),
                    product_sub: "20030107".to_string(),
                    user_agent_data: None,
                    do_not_track: None,
                    app_code_name: Some("Mozilla".to_string()),
                    app_name: Some("Netscape".to_string()),
                    app_version: None,
                    oscpu: None,
                    vendor_sub: None,
                    max_touch_points: Some(0),
                    product: Some("Gecko".to_string()),
                    extra_properties: None,
                },
                screen: ScreenFingerprint {
                    width: 1920,
                    height: 1080,
                    avail_width: 1920,
                    avail_height: 1040,
                    color_depth: 24,
                    pixel_depth: 24,
                    device_pixel_ratio: 1.0,
                    inner_width: 1920,
                    inner_height: 937,
                    avail_top: None,
                    avail_left: None,
                    outer_width: None,
                    outer_height: None,
                    screen_x: None,
                    page_x_offset: None,
                    page_y_offset: None,
                    client_width: None,
                    client_height: None,
                    has_hdr: None,
                },
                video_card: None,
                audio_codecs: None,
                video_codecs: None,
                battery: None,
                multimedia_devices: None,
                plugins_data: None,
                fonts: None,
                mock_web_rtc: None,
                slim: None,
            },
        }
    }


    #[test]
    fn browser_profile_roundtrip() {
        let profile = make_test_profile();
        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: BrowserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile.generated_at, deserialized.generated_at);
        assert_eq!(profile.dataset_version, deserialized.dataset_version);
        assert_eq!(profile.browser.version, deserialized.browser.version);
    }

    #[test]
    fn browser_family_serializes_lowercase() {
        let family = BrowserFamily::Chrome;
        let json = serde_json::to_string(&family).unwrap();
        assert_eq!(json, "\"chrome\"");
    }

    #[test]
    fn json_has_camel_case_fields() {
        let profile = make_test_profile();
        let json = serde_json::to_value(&profile).unwrap();
        assert!(json.get("generatedAt").is_some(), "generatedAt must be camelCase");
        assert!(json.get("datasetVersion").is_some(), "datasetVersion must be camelCase");
        assert!(json.get("operatingSystem").is_some(), "operatingSystem must be camelCase");
    }
}

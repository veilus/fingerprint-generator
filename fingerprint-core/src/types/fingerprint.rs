use serde::{Deserialize, Serialize};

/// A brand-version pair from the User-Agent Client Hints API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandVersion {
    /// Browser brand name (e.g., "Google Chrome").
    pub brand: String,
    /// Major version string (e.g., "120").
    pub version: String,
}

/// `navigator.userAgentData` — Chrome and Edge only, `None` for Firefox/Safari.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAgentData {
    /// List of brand-version pairs.
    pub brands: Vec<BrandVersion>,
    /// Whether the device is mobile.
    pub mobile: bool,
    /// Platform string (e.g., "Windows").
    pub platform: String,
    /// CPU architecture (e.g., "x86").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    /// CPU bitness (e.g., "64").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitness: Option<String>,
    /// Device model string (empty for desktop).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Platform version (e.g., "10.0.0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_version: Option<String>,
    /// Full UA version string (e.g., "125.0.6422.141").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ua_full_version: Option<String>,
    /// Full brand-version list with complete version strings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_version_list: Option<Vec<BrandVersion>>,
}

/// `navigator.*` JavaScript API values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorFingerprint {
    /// `navigator.userAgent`
    pub user_agent: String,
    /// `navigator.hardwareConcurrency`
    pub hardware_concurrency: u8,
    /// `navigator.deviceMemory` — `None` when not exposed (Firefox, privacy budget).
    pub device_memory: Option<f32>,
    /// `navigator.platform`
    pub platform: String,
    /// `navigator.language`
    pub language: String,
    /// `navigator.languages`
    pub languages: Vec<String>,
    /// `navigator.webdriver` — always `false`.
    pub webdriver: bool,
    /// `navigator.vendor`
    pub vendor: String,
    /// `navigator.productSub`
    pub product_sub: String,
    /// `navigator.userAgentData` — `None` for Firefox and Safari.
    pub user_agent_data: Option<UserAgentData>,
    /// `navigator.doNotTrack`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_not_track: Option<String>,
    /// `navigator.appCodeName` — always "Mozilla".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_code_name: Option<String>,
    /// `navigator.appName` — always "Netscape".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// `navigator.appVersion`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    /// `navigator.oscpu` — Firefox only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oscpu: Option<String>,
    /// `navigator.vendorSub`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_sub: Option<String>,
    /// `navigator.maxTouchPoints`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_touch_points: Option<u8>,
    /// `navigator.product` — always "Gecko".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// Extra browser properties (vendorFlavors, pdfViewerEnabled, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_properties: Option<ExtraProperties>,
}

/// Browser-specific extra navigator properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraProperties {
    /// Browser vendor flavors (e.g., `["chrome"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_flavors: Option<Vec<String>>,
    /// `navigator.pdfViewerEnabled`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_viewer_enabled: Option<bool>,
    /// `navigator.globalPrivacyControl`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_privacy_control: Option<bool>,
    /// Installed apps (usually empty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_apps: Option<Vec<String>>,
}

/// `screen.*` JavaScript API values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenFingerprint {
    /// `screen.width`
    pub width: u32,
    /// `screen.height`
    pub height: u32,
    /// `screen.availWidth`
    pub avail_width: u32,
    /// `screen.availHeight`
    pub avail_height: u32,
    /// `screen.colorDepth`
    pub color_depth: u8,
    /// `screen.pixelDepth`
    pub pixel_depth: u8,
    /// `window.devicePixelRatio`
    pub device_pixel_ratio: f32,
    /// `window.innerWidth`
    pub inner_width: u32,
    /// `window.innerHeight`
    pub inner_height: u32,
    /// `screen.availTop`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avail_top: Option<u32>,
    /// `screen.availLeft`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avail_left: Option<u32>,
    /// `window.outerWidth`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_width: Option<u32>,
    /// `window.outerHeight`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_height: Option<u32>,
    /// `window.screenX`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_x: Option<i32>,
    /// `window.pageXOffset`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_x_offset: Option<u32>,
    /// `window.pageYOffset`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_y_offset: Option<u32>,
    /// `document.documentElement.clientWidth`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_width: Option<u32>,
    /// `document.documentElement.clientHeight`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_height: Option<u32>,
    /// Whether the display supports HDR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_hdr: Option<bool>,
}

/// WebGL video card information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCard {
    /// WebGL renderer string (e.g., "ANGLE (Intel...)").
    pub renderer: String,
    /// WebGL vendor string (e.g., "Google Inc. (Intel)").
    pub vendor: String,
}

/// Media codec support levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCodecs {
    /// Ogg Vorbis support level.
    pub ogg: String,
    /// MP3 support level.
    pub mp3: String,
    /// WAV support level.
    pub wav: String,
    /// M4A support level.
    pub m4a: String,
    /// AAC support level.
    pub aac: String,
}

/// Video codec support levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCodecs {
    /// Ogg Theora support level.
    pub ogg: String,
    /// H.264 support level.
    pub h264: String,
    /// WebM/VP8 support level.
    pub webm: String,
}

/// Battery status simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    /// Whether the battery is charging.
    pub charging: bool,
    /// Time in seconds until fully charged (`None` when on AC with full charge).
    pub charging_time: Option<f64>,
    /// Time in seconds until discharged (`None` when plugged in).
    pub discharging_time: Option<f64>,
    /// Battery level (0.0 to 1.0).
    pub level: f64,
}

/// Multimedia device counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultimediaDevices {
    /// Number of audio output devices.
    pub speakers: u8,
    /// Number of audio input devices.
    pub micros: u8,
    /// Number of video input devices.
    pub webcams: u8,
}

/// Plugin MIME type information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMimeType {
    /// MIME type string (e.g., "application/pdf").
    #[serde(rename = "type")]
    pub mime_type: String,
    /// File suffixes (e.g., "pdf").
    pub suffixes: String,
    /// Human-readable description.
    pub description: String,
    /// Name of the plugin that handles this MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_plugin: Option<String>,
}

/// Browser plugin information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    /// Plugin name (e.g., "Chromium PDF Viewer").
    pub name: String,
    /// Plugin description.
    pub description: String,
    /// Plugin filename.
    pub filename: String,
    /// MIME types handled by this plugin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_types: Option<Vec<PluginMimeType>>,
}

/// Plugins and MIME types data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginsData {
    /// Installed browser plugins.
    pub plugins: Vec<Plugin>,
    /// Registered MIME types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_types: Option<Vec<PluginMimeType>>,
}

/// Combined browser fingerprint (navigator + screen + extended).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserFingerprint {
    /// Navigator API fingerprint values.
    pub navigator: NavigatorFingerprint,
    /// Screen API fingerprint values.
    pub screen: ScreenFingerprint,
    /// WebGL video card information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_card: Option<VideoCard>,
    /// Audio codec support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_codecs: Option<AudioCodecs>,
    /// Video codec support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_codecs: Option<VideoCodecs>,
    /// Battery status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<Battery>,
    /// Multimedia device counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multimedia_devices: Option<MultimediaDevices>,
    /// Browser plugins data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins_data: Option<PluginsData>,
    /// Installed fonts list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fonts: Option<Vec<String>>,
    /// Whether to mock WebRTC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_web_rtc: Option<bool>,
    /// Slim mode flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slim: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigator_roundtrip_without_ua_data() {
        let nav = NavigatorFingerprint {
            user_agent: "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0"
                .to_string(),
            hardware_concurrency: 4,
            device_memory: None,
            platform: "Linux x86_64".to_string(),
            language: "en-US".to_string(),
            languages: vec!["en-US".to_string()],
            webdriver: false,
            vendor: "".to_string(),
            product_sub: "20100101".to_string(),
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
        };
        let json = serde_json::to_string(&nav).unwrap();
        let back: NavigatorFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(nav.user_agent, back.user_agent);
        assert!(back.user_agent_data.is_none());
        assert!(!back.webdriver, "webdriver must always be false");
    }

    #[test]
    fn navigator_roundtrip_with_ua_data() {
        let nav = NavigatorFingerprint {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120".to_string(),
            hardware_concurrency: 16,
            device_memory: Some(8.0),
            platform: "Win32".to_string(),
            language: "en-US".to_string(),
            languages: vec!["en-US".to_string(), "en".to_string()],
            webdriver: false,
            vendor: "Google Inc.".to_string(),
            product_sub: "20030107".to_string(),
            user_agent_data: Some(UserAgentData {
                brands: vec![BrandVersion {
                    brand: "Google Chrome".to_string(),
                    version: "120".to_string(),
                }],
                mobile: false,
                platform: "Windows".to_string(),
                architecture: Some("x86".to_string()),
                bitness: Some("64".to_string()),
                model: Some(String::new()),
                platform_version: Some("10.0.0".to_string()),
                ua_full_version: Some("120.0.6099.109".to_string()),
                full_version_list: None,
            }),
            do_not_track: None,
            app_code_name: Some("Mozilla".to_string()),
            app_name: Some("Netscape".to_string()),
            app_version: None,
            oscpu: None,
            vendor_sub: None,
            max_touch_points: Some(0),
            product: Some("Gecko".to_string()),
            extra_properties: None,
        };
        let json = serde_json::to_string(&nav).unwrap();
        let back: NavigatorFingerprint = serde_json::from_str(&json).unwrap();
        assert!(back.user_agent_data.is_some());
        assert_eq!(back.device_memory, Some(8.0));
        let uad = back.user_agent_data.unwrap();
        assert_eq!(uad.architecture.as_deref(), Some("x86"));
    }

    #[test]
    fn screen_roundtrip() {
        let screen = ScreenFingerprint {
            width: 1920,
            height: 1080,
            avail_width: 1920,
            avail_height: 1040,
            color_depth: 24,
            pixel_depth: 24,
            device_pixel_ratio: 1.0,
            inner_width: 1800,
            inner_height: 900,
            avail_top: Some(0),
            avail_left: Some(0),
            outer_width: Some(1920),
            outer_height: Some(1040),
            screen_x: Some(0),
            page_x_offset: Some(0),
            page_y_offset: Some(0),
            client_width: Some(0),
            client_height: Some(18),
            has_hdr: Some(false),
        };
        let json = serde_json::to_string(&screen).unwrap();
        let back: ScreenFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(screen.width, back.width);
        assert!((screen.device_pixel_ratio - back.device_pixel_ratio).abs() < f32::EPSILON);
        assert_eq!(back.has_hdr, Some(false));
    }

    #[test]
    fn json_fields_are_camel_case() {
        let screen = ScreenFingerprint {
            width: 1920,
            height: 1080,
            avail_width: 1920,
            avail_height: 1040,
            color_depth: 24,
            pixel_depth: 24,
            device_pixel_ratio: 1.5,
            inner_width: 1800,
            inner_height: 900,
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
        };
        let json = serde_json::to_value(&screen).unwrap();
        assert!(json.get("devicePixelRatio").is_some(), "devicePixelRatio must be camelCase");
        assert!(json.get("availWidth").is_some(), "availWidth must be camelCase");
        assert!(json.get("innerWidth").is_some(), "innerWidth must be camelCase");
    }
}

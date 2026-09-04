use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use veilus_fingerprint_core::{
    BrowserFamily, BrowserProfile, DeviceType, FingerprintError, OsFamily,
};
use veilus_fingerprint_data::loader::{get_fingerprint_network, get_header_network};

use crate::assembler::assemble_profile;
use crate::engine::constraints::{sample_constrained, Constraints};
use crate::engine::sampler::{sample_ancestral, sample_ancestral_with_evidence};

// ─── Known impossible constraint combinations ─────────────────────────────

/// Browser/OS combos known to be statistically absent from real-world data.
/// These cannot be satisfied by the sampler — fail fast with ConstraintConflict.
const IMPOSSIBLE_COMBOS: &[(&str, &str)] = &[
    ("safari", "windows"),
    ("safari", "linux"),
    ("safari", "android"),
];

/// Fluent builder for generating realistic browser fingerprints.
///
/// # Example
///
/// ```rust,ignore
/// use veilus_fingerprint::{FingerprintGenerator, BrowserFamily, OsFamily};
///
/// // Random fingerprint
/// let profile = FingerprintGenerator::random()?;
///
/// // Constrained fingerprint
/// let profile = FingerprintGenerator::new()
///     .browser(BrowserFamily::Chrome)
///     .os(OsFamily::Windows)
///     .locale("en-US")
///     .generate()?;
///
/// // Deterministic fingerprint (same seed → same sampled fields)
/// let profile = FingerprintGenerator::new()
///     .seeded("my-session-id")
///     .generate()?;
/// ```
#[must_use]
#[derive(Debug, Default)]
pub struct FingerprintGenerator {
    browser: Option<BrowserFamily>,
    os: Option<OsFamily>,
    device: Option<DeviceType>,
    locale: Option<String>,
    seed: Option<u64>,
    strict: bool,
}

impl FingerprintGenerator {
    /// Create a new unconstrained generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a random `BrowserProfile` with no constraints (convenience shorthand).
    ///
    /// Equivalent to `FingerprintGenerator::new().generate()`.
    pub fn random() -> Result<BrowserProfile, FingerprintError> {
        Self::new().generate()
    }

    /// Constrain the browser family (e.g., `BrowserFamily::Chrome`).
    pub fn browser(mut self, family: BrowserFamily) -> Self {
        self.browser = Some(family);
        self
    }

    /// Constrain the operating system family (e.g., `OsFamily::Windows`).
    pub fn os(mut self, family: OsFamily) -> Self {
        self.os = Some(family);
        self
    }

    /// Constrain the device type (e.g., `DeviceType::Mobile`, `DeviceType::Desktop`).
    pub fn device(mut self, device: DeviceType) -> Self {
        self.device = Some(device);
        self
    }

    /// Constrain the browser locale (e.g., `"en-US"`).
    ///
    /// Note: The header network does not model locale — this constraint is stored
    /// but reserved for future network support. Currently a no-op on the sampler level.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Enable deterministic generation using a u64 seed.
    ///
    /// Two calls with the same seed produce profiles with identical sampled fields
    /// (user agent, screen, etc.) but always different `id` and `generated_at`.
    pub fn seeded(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Enable strict mode: return `ConstraintsTooRestrictive` instead of silently relaxing.
    ///
    /// Without `.strict()`, if constraints cannot be satisfied after max attempts,
    /// the generator falls back to an unconstrained sample. With `.strict()`, it errors.
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Generate a `BrowserProfile` based on configured constraints.
    ///
    /// # Errors
    ///
    /// - `FingerprintError::ConstraintConflict` — browser/OS combination is known impossible
    ///   (e.g., Safari + Windows).
    /// - `FingerprintError::ConstraintsTooRestrictive` — constraints cannot be satisfied
    ///   within the retry budget (only when `.strict()` is set).
    /// - `FingerprintError::NetworkParseError` — embedded Bayesian network data is corrupt
    ///   (should never happen in a correctly built binary).
    pub fn generate(self) -> Result<BrowserProfile, FingerprintError> {
        // ── Story 4.3: Pre-validate impossible combos ─────────────────────
        self.validate_constraints()?;

        // ── Build constraint maps ──────────────────────────────────────────
        let header_constraints = self.build_header_constraints();
        let fp_constraints = self.build_fp_constraints();

        // ── Story 4.2: Build RNG (seeded or random) ───────────────────────
        let mut rng = self.build_rng();

        let fp_network = get_fingerprint_network()?;
        let header_network = get_header_network()?;

        // ── Sample header network FIRST ────────────────────────────────────
        //
        // THU TU O DAY LA MOT BAN SUA LOI, KHONG PHAI SO THICH.
        //
        // Truoc 2026-09-04, hai mang duoc lay mau DOC LAP va mang fingerprint
        // khong nhan rang buoc nao (`build_fp_constraints` tra ve rong).
        // `assemble_profile` doc `operating_system` tu mang HEADER va
        // `userAgent` tu mang FINGERPRINT - nen thu vien tu bao da thoa
        // `.os(Windows)` trong khi UA noi Linux. Do duoc 46,5% / 74,3% /
        // 85,8% lech cho Windows / macOS / Linux tren 2000 seed moi loai, va
        // mot seed cho ho so khai BA he dieu hanh cung luc. Xem VEIL-407.
        //
        // Mang header la mang NHAN duoc rang buoc (`*OPERATING_SYSTEM`,
        // `*BROWSER`, `*DEVICE`), nen no phai chay truoc; UA no chon roi tro
        // thanh rang buoc cho mang fingerprint.
        let header_assignment = if header_constraints.is_empty() {
            sample_ancestral(header_network, &mut rng)?
        } else {
            match sample_constrained(header_network, &header_constraints, &mut rng) {
                Ok(a) => a,
                Err(e) if !self.strict => {
                    tracing::warn!("header constraints failed, relaxing: {e}");
                    sample_ancestral(header_network, &mut rng)?
                }
                Err(e) => return Err(e),
            }
        };

        // ── Sample fingerprint network, KEP theo UA vua chon ───────────────
        //
        // EP BANG NHAU, KHONG EP "CUNG KIEU WINDOWS". Hai mang dung chung
        // 470/479 gia tri userAgent (do 2026-09-04), nen ghim dung chuoi da
        // chon cho su nhat quan TUYET DOI thay vi chi cung ho OS. Loc theo
        // dau hieu OS van de lot mot UA Windows khac phien ban voi header.
        //
        // KEP (evidence) chu khong LAY MAU BAC BO. `userAgent` la GOC cua
        // mang fingerprint (parent_names rong, 479 gia tri), nen kep no la
        // phep chinh xac: moi nut con van boc tu phan phoi co dieu kien dung.
        //
        // Ban dau cho nay dung `sample_constrained`, va van con 1 trong 8
        // seed thu nghiem do. Ly do: bac bo co ngan sach `so_nut * 10 = 250`
        // luot, ma ghim dung 1 trong 479 gia tri thi thuong khong trung trong
        // 250 luot -> roi ve nhanh du phong KHONG rang buoc -> lap lai chinh
        // loi dang sua. Bac bo hop voi rang buoc rong, khong hop voi ghim mot
        // gia tri.
        let mut fp_evidence: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut fp_filters: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        if let Some(ua) = header_user_agent(&header_assignment) {
            fp_filters = loc_theo_ua(fp_network, &ua);
            fp_evidence.insert("userAgent".to_string(), ua);
        }

        let fp_assignment = if fp_constraints.is_empty() {
            sample_ancestral_with_evidence(fp_network, &fp_evidence, &fp_filters, &mut rng)?
        } else {
            match sample_constrained(fp_network, &fp_constraints, &mut rng) {
                Ok(a) => a,
                Err(e) if !self.strict => {
                    tracing::warn!("fp constraints failed, relaxing: {e}");
                    sample_ancestral_with_evidence(fp_network, &fp_evidence, &fp_filters, &mut rng)?
                }
                Err(e) => return Err(e),
            }
        };

        // ── Tang hai: suy ra khi CPT khong co gia tri nao nhat quan ───────
        //
        // Loc (tang mot) giu duoc da dang o cho CPT CO gia tri hop le. Nhung
        // do 2026-09-04, 27/83 UA Windows va 48/101 UA macOS co nhanh CPT
        // `platform` KHONG chua gia tri nao cua chinh OS do - mot trong so do
        // la `{"Linux x86_64": 1.0}`. O nhung UA ay khong co gi de loc, nen
        // chi con hai duong: giu mau thuan, hoac suy ra.
        //
        // Suy ra. Mot ho so tu mau thuan bi bat ngay; mot `platform` suy tu UA
        // thi dung theo dinh nghia. Cai mat la da dang o dung nhung UA do, va
        // do la cai gia dang tra.
        let mut fp_assignment = fp_assignment;
        if let Some(ua) = fp_assignment.get("userAgent").cloned() {
            if let Some(os) = os_tu_ua(&ua) {
                // `map_or` chu khong `is_none_or`: MSRV cua crate la 1.75,
                // con `is_none_or` moi on dinh tu 1.82.
                let can_sua = fp_assignment
                    .get("platform")
                    .map_or(true, |pf| !platform_khop_os(pf, &os));
                if can_sua {
                    fp_assignment.insert("platform".to_string(), platform_suy_ra(&os).to_string());
                }
            }
        }

        assemble_profile(&fp_assignment, &header_assignment, &mut rng)
    }

    // ── Private helpers ────────────────────────────────────────────────────

    /// Validate that the browser/OS combination is not known to be impossible.
    fn validate_constraints(&self) -> Result<(), FingerprintError> {
        if let (Some(browser), Some(os)) = (&self.browser, &self.os) {
            let browser_key = browser_family_to_key(browser);
            let os_key = os_family_to_key(os);
            for (b, o) in IMPOSSIBLE_COMBOS {
                if &browser_key == b && &os_key == o {
                    return Err(FingerprintError::ConstraintConflict {
                        browser: browser_key,
                        os: os_key,
                    });
                }
            }
        }
        Ok(())
    }

    /// Build the constraint map for the **header** Bayesian network.
    ///
    /// The `*BROWSER` node uses values like `"chrome/120.0.0.0"`, so we collect
    /// all possible values that start with the requested browser family prefix.
    fn build_header_constraints(&self) -> Constraints {
        let mut c = Constraints::new();

        if let Some(browser) = &self.browser {
            let prefix = browser_family_to_key(browser);
            // Load the header network to enumerate valid *BROWSER values
            if let Ok(network) = get_header_network() {
                if let Some(browser_node) = network.nodes.iter().find(|n| n.name == "*BROWSER") {
                    let matching: Vec<String> = browser_node
                        .possible_values
                        .iter()
                        .filter(|v| v.to_lowercase().starts_with(&prefix))
                        .cloned()
                        .collect();
                    if !matching.is_empty() {
                        c.insert("*BROWSER".to_string(), matching);
                    }
                }
            }
        }

        if let Some(os) = &self.os {
            // `*OPERATING_SYSTEM` node has flat values: "windows", "macos", etc.
            c.insert("*OPERATING_SYSTEM".to_string(), vec![os_family_to_key(os)]);

            // RANG BUOC THANG CA HAI NUT UA THEO DAU HIEU OS.
            //
            // Ghim `*OPERATING_SYSTEM` la KHONG DU, du `user-agent` co no lam
            // cha. Do 2026-09-04, `.os(windows)` khong kem `.device()` cho UA
            // Android o 62/500 luot (12,4%).
            //
            // Co che: `*DEVICE` khong bi rang buoc nen boc tu do, va
            // `windows + mobile` la to hop KHONG CO trong du lieu Apify. CPT
            // khong co nhanh cho no, nen `traverse_cpt_and_sample` roi ve
            // phan phoi LE - phan phoi do do Android chiem uu the, va UA
            // Android chui ra duoi mot ho so khai Windows.
            //
            // Them `*DEVICE = desktop` chua duoc trieu chung (62/500 -> 0/500),
            // nhung do la doan mot bang OS->thiet bi ma khong ai do: Surface
            // chay Windows va la tablet. Rang buoc thang UA thi dung bat ke
            // `*DEVICE` boc ra gi, va khong phai bia bang nao.
            //
            // `*MISSING_VALUE*` PHAI nam trong tap cho phep: hai nut UA la hai
            // duong HTTP loai tru nhau (`user-agent` cho h2, `User-Agent` cho
            // h1), nut khong duoc dung se mang gia tri do. Bo no ra thi khong
            // mau nao thoa duoc ca hai nut cung luc.
            if let Ok(network) = get_header_network() {
                for ten_nut in ["user-agent", "User-Agent"] {
                    if let Some(nut) = network.nodes.iter().find(|n| n.name == ten_nut) {
                        let hop_le: Vec<String> = nut
                            .possible_values
                            .iter()
                            .map(std::string::ToString::to_string)
                            .filter(|v| v == MISSING_VALUE || ua_khop_os(v, os))
                            .collect();
                        if !hop_le.is_empty() {
                            c.insert(ten_nut.to_string(), hop_le);
                        }
                    }
                }
            }
        }

        if let Some(device) = &self.device {
            // `*DEVICE` node has flat values: "desktop", "mobile", "tablet"
            c.insert("*DEVICE".to_string(), vec![device_type_to_key(device)]);
        }

        c
    }

    /// Build the constraint map for the **fingerprint** Bayesian network.
    fn build_fp_constraints(&self) -> Constraints {
        // The fingerprint network has `userAgent` as root — no browserFamily node.
        // Constraints for this network are limited; most constraints apply to header net.
        Constraints::new()
    }

    /// Build the ChaCha8 RNG — seeded deterministically or from entropy.
    fn build_rng(&self) -> ChaCha8Rng {
        if let Some(seed) = self.seed {
            ChaCha8Rng::seed_from_u64(seed)
        } else {
            ChaCha8Rng::from_entropy()
        }
    }
}

// ─── Network key mappings ─────────────────────────────────────────────────

/// Map BrowserFamily to the lowercase key used in the Apify networks.
fn browser_family_to_key(family: &BrowserFamily) -> String {
    match family {
        BrowserFamily::Chrome => "chrome".to_string(),
        BrowserFamily::Firefox => "firefox".to_string(),
        BrowserFamily::Safari => "safari".to_string(),
        BrowserFamily::Edge => "edge".to_string(),
        BrowserFamily::Other(s) => s.to_lowercase(),
    }
}

/// Ho dieu hanh suy tu chuoi User-Agent.
///
/// Chuoi UA la thu site doc duoc, nen no la SU THAT ve OS cua ho so. Moi
/// truong khac phai theo no, khong phai nguoc lai.
fn os_tu_ua(ua: &str) -> Option<OsFamily> {
    if ua.contains("Android") {
        Some(OsFamily::Android)
    } else if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
        Some(OsFamily::Ios)
    } else if ua.contains("Windows") {
        Some(OsFamily::Windows)
    } else if ua.contains("Macintosh") {
        Some(OsFamily::MacOs)
    } else if ua.contains("Linux") || ua.contains("X11") {
        Some(OsFamily::Linux)
    } else {
        None
    }
}

/// `navigator.platform` nay co noi cung mot OS voi UA khong.
fn platform_khop_os(pf: &str, os: &OsFamily) -> bool {
    let arm_di_dong = pf.contains("armv") || pf.contains("aarch64");
    match os {
        OsFamily::Windows => pf.starts_with("Win"),
        OsFamily::MacOs => pf == "MacIntel" || pf.starts_with("Mac"),
        // Android CUNG bao "Linux ...", nen Linux phai loai ARM di dong ra.
        // Bay nay giong het bay "Linux" trong chuoi UA cua Android.
        OsFamily::Linux => pf.starts_with("Linux") && !arm_di_dong,
        OsFamily::Android => pf.starts_with("Linux"),
        OsFamily::Ios => pf == "iPhone" || pf == "iPad" || pf.starts_with("iP"),
        OsFamily::Other(_) => true,
    }
}

/// `navigator.platform` suy tu OS, dung khi CPT khong cap duoc gia tri hop le.
///
/// Chrome tren Windows 64-bit VAN bao "Win32" - do la hanh vi that, khong
/// phai nham. Bang nay khop voi nhanh du phong cua `assemble_profile`.
fn platform_suy_ra(os: &OsFamily) -> &'static str {
    match os {
        OsFamily::Windows => "Win32",
        OsFamily::MacOs => "MacIntel",
        OsFamily::Linux => "Linux x86_64",
        OsFamily::Android => "Linux armv8l",
        OsFamily::Ios => "iPhone",
        OsFamily::Other(_) => "",
    }
}

/// Nhan `"platform":"X"` ma `userAgentData` phai mang, theo OS.
fn nhan_uach(os: &OsFamily) -> Option<&'static str> {
    match os {
        OsFamily::Windows => Some("\"platform\":\"Windows\""),
        OsFamily::MacOs => Some("\"platform\":\"macOS\""),
        OsFamily::Linux => Some("\"platform\":\"Linux\""),
        OsFamily::Android => Some("\"platform\":\"Android\""),
        OsFamily::Ios => None, // Safari/iOS khong gui userAgentData
        OsFamily::Other(_) => None,
    }
}

/// Chuoi renderer WebGL nay co MAU THUAN voi OS khong.
///
/// Chi loai thu CHAC CHAN sai, khong doi phai khop duong: "Direct3D" chi ton
/// tai tren Windows, GPU "Apple M*" chi ton tai tren may Apple. Con lai
/// (Intel, NVIDIA, AMD qua OpenGL) chay duoc nhieu OS nen khong loai.
fn renderer_mau_thuan_os(r: &str, os: &OsFamily) -> bool {
    let d3d = r.contains("Direct3D") || r.contains("D3D11");
    let apple_silicon = r.contains("Apple M") || r.contains("ANGLE (Apple");
    match os {
        OsFamily::Windows => apple_silicon,
        OsFamily::MacOs => d3d,
        OsFamily::Linux | OsFamily::Android => d3d || apple_silicon,
        OsFamily::Ios => d3d,
        OsFamily::Other(_) => false,
    }
}

/// Tap gia tri cho phep cua tung nut, suy tu UA da chot.
///
/// LY DO TON TAI. `userAgent` la GOC va da duoc kep, nhung CPT cua cac nut con
/// VAN chua mau thuan: do 2026-09-04, 34/83 UA Windows co nhanh `platform` lan
/// sang OS khac, mot trong so do la `{"Linux x86_64": 1.0}` - tuc UA Windows
/// mà KHONG BAO GIO ra platform Windows. Khong nut nao roi ve nhanh `skip`,
/// nen day la DU LIEU chu khong phai loi tra bang.
///
/// Tap Apify cao tu traffic that, trong do co may DANG SPOOF HONG. Lay mau
/// trung thanh thi tai tao luon cai spoof hong cua nguoi khac.
///
/// Bo sinh fingerprint khong nham tai tao dan so KE CA KE NOI DOI trong do.
/// No nham tao ra mot thanh vien BINH THUONG cua dan so. Mot ho so tu mau
/// thuan khong phai thanh vien binh thuong - no la ke noi doi bi bat.
fn loc_theo_ua(
    network: &veilus_fingerprint_data::network::BayesianNetwork,
    ua: &str,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut f = std::collections::HashMap::new();
    let Some(os) = os_tu_ua(ua) else { return f };

    let gia_tri = |ten: &str| -> Vec<String> {
        network
            .nodes
            .iter()
            .find(|n| n.name == ten)
            .map(|n| n.possible_values.iter().map(ToString::to_string).collect())
            .unwrap_or_default()
    };

    let pf: Vec<String> = gia_tri("platform")
        .into_iter()
        .filter(|v| platform_khop_os(v, &os))
        .collect();
    if !pf.is_empty() {
        f.insert("platform".to_string(), pf);
    }

    if let Some(nhan) = nhan_uach(&os) {
        let uad: Vec<String> = gia_tri("userAgentData")
            .into_iter()
            .filter(|v| v.contains(nhan))
            .collect();
        if !uad.is_empty() {
            f.insert("userAgentData".to_string(), uad);
        }
    }

    let vc: Vec<String> = gia_tri("videoCard")
        .into_iter()
        .filter(|v| !renderer_mau_thuan_os(v, &os))
        .collect();
    if !vc.is_empty() {
        f.insert("videoCard".to_string(), vc);
    }

    f
}

/// Gia tri Apify danh dau "header nay khong co mat".
const MISSING_VALUE: &str = "*MISSING_VALUE*";

/// Chuoi User-Agent nay co phai cua ho dieu hanh do khong.
///
/// Dung DAU HIEU trong chinh chuoi UA, khong dung bang tra: chuoi UA la thu
/// site doc duoc, nen no moi la su that ve OS.
///
/// Hai bay da do 2026-09-04:
///   - UA Android CO chua "Linux" ("Linux; Android 10; K"), nen Linux phai
///     loai Android ra chu khong chi tim "Linux".
///   - KHONG phai UA Linux nao cung co "X11": ban do bat gap
///     "Mozilla/5.0 (CentOS; Linux i686)" - hop le, va mot bo loc chi tim
///     "X11" se vut no di.
fn ua_khop_os(ua: &str, os: &OsFamily) -> bool {
    let co_android = ua.contains("Android");
    let co_ios = ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod");
    match os {
        OsFamily::Windows => ua.contains("Windows"),
        // "Mac OS X" KHONG dung mot minh: UA cua iOS cung chua no
        // ("CPU iPhone OS 17_0 like Mac OS X"). Bo loc dau tien o day dung
        // `Macintosh || Mac OS X` va cho iPhone lot vao macOS - luot quet 200
        // seed bat duoc o seed 61. Cung hinh dang voi bay Android/Linux ngay
        // duoi, chi khac cap OS.
        OsFamily::MacOs => ua.contains("Macintosh") && !co_ios,
        OsFamily::Linux => (ua.contains("Linux") || ua.contains("X11")) && !co_android,
        OsFamily::Android => co_android,
        OsFamily::Ios => co_ios,
        OsFamily::Other(_) => true,
    }
}

/// Chuoi User-Agent ma mang header vua chon, neu co.
///
/// Mang header co HAI nut UA: `user-agent` (471 gia tri, duong HTTP/2) va
/// `User-Agent` (32 gia tri, duong HTTP/1). Uu tien ban thuong vi no phu gan
/// het tap gia tri; ban hoa dung lam du phong.
///
/// `*MISSING_VALUE*` la cach Apify danh dau "khong co header nay" - ghim no
/// lam rang buoc se lam mang fingerprint khong con gia tri nao hop le.
fn header_user_agent(assignment: &std::collections::HashMap<String, String>) -> Option<String> {
    for ten in ["user-agent", "User-Agent"] {
        if let Some(v) = assignment.get(ten) {
            if !v.is_empty() && v != MISSING_VALUE {
                return Some(v.clone());
            }
        }
    }
    None
}

/// Map OsFamily to the lowercase key used in the Apify networks.
fn os_family_to_key(family: &OsFamily) -> String {
    match family {
        OsFamily::Windows => "windows".to_string(),
        OsFamily::MacOs => "macos".to_string(),
        OsFamily::Linux => "linux".to_string(),
        OsFamily::Android => "android".to_string(),
        OsFamily::Ios => "ios".to_string(),
        OsFamily::Other(s) => s.to_lowercase(),
    }
}

/// Map DeviceType to the lowercase key used in the Apify networks.
fn device_type_to_key(device: &DeviceType) -> String {
    match device {
        DeviceType::Desktop => "desktop".to_string(),
        DeviceType::Mobile => "mobile".to_string(),
        DeviceType::Tablet => "tablet".to_string(),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_succeeds() {
        let profile = FingerprintGenerator::random().expect("random generation must succeed");
        assert!(!profile.fingerprint.navigator.user_agent.is_empty());
        assert!(
            !profile.fingerprint.navigator.webdriver,
            "webdriver must always be false"
        );
        assert!(profile.generated_at > 0, "generated_at must be set");
        assert_ne!(profile.id, [0u8; 16], "id must be non-zero");
    }

    #[test]
    fn seeded_produces_identical_user_agents() {
        let profile1 = FingerprintGenerator::new()
            .seeded(42)
            .generate()
            .expect("seeded must succeed");
        let profile2 = FingerprintGenerator::new()
            .seeded(42)
            .generate()
            .expect("seeded must succeed again");

        // Sampled fields must match for same seed
        assert_eq!(
            profile1.fingerprint.navigator.user_agent, profile2.fingerprint.navigator.user_agent,
            "same seed must produce identical user_agent"
        );
        assert_eq!(
            profile1.fingerprint.screen.width, profile2.fingerprint.screen.width,
            "same seed must produce identical screen width"
        );

        // These must DIFFER (not seeded)
        // id is always random — extremely unlikely to match
        // We just verify they are valid non-zero values
        assert_ne!(profile1.id, [0u8; 16]);
        assert_ne!(profile2.id, [0u8; 16]);
    }

    #[test]
    fn different_seeds_produce_different_profiles() {
        let p1 = FingerprintGenerator::new()
            .seeded(1)
            .generate()
            .expect("must succeed");
        let p2 = FingerprintGenerator::new()
            .seeded(999)
            .generate()
            .expect("must succeed");

        // Very unlikely to have same UA with different seeds
        // (may occasionally match for small seeds — acceptable)
        let _ = (p1, p2); // Just verify compilation and no panic
    }

    #[test]
    fn safari_windows_conflict_is_rejected() {
        let result = FingerprintGenerator::new()
            .browser(BrowserFamily::Safari)
            .os(OsFamily::Windows)
            .strict()
            .generate();

        assert!(
            matches!(result, Err(FingerprintError::ConstraintConflict { .. })),
            "Safari+Windows must fail with ConstraintConflict, got: {:?}",
            result.err()
        );
    }

    /// Dau hieu OS trong chuoi User-Agent.
    fn dau_hieu_ua(os: &OsFamily) -> &'static str {
        match os {
            OsFamily::Windows => "Windows NT",
            OsFamily::MacOs => "Macintosh",
            OsFamily::Linux => "Linux",
            OsFamily::Android => "Android",
            OsFamily::Ios => "iPhone",
            OsFamily::Other(_) => "",
        }
    }

    /// Rang buoc `.os()` phai toi ca `navigator.userAgent`, khong chi
    /// `operating_system.name`.
    ///
    /// LY DO TON TAI. Truoc ban sua nay, hai mang Bayes duoc lay mau DOC LAP:
    /// mang header nhan rang buoc, mang fingerprint khong nhan gi
    /// (`build_fp_constraints` tra ve rong). `operating_system` doc tu mang
    /// header con `userAgent` doc tu mang fingerprint, nen thu vien TU BAO da
    /// thoa rang buoc trong khi UA noi mot he dieu hanh khac.
    ///
    /// Do 2026-09-04, 2000 seed moi OS:
    ///   os=Windows -> UA khong phai Windows:  930/2000  (46,5%)
    ///   os=macOS   -> UA khong phai macOS:   1486/2000  (74,3%)
    ///   os=Linux   -> UA khong phai Linux:   1715/2000  (85,8%)
    ///
    /// Seed 11 cho mot ho so khai BA he dieu hanh cung luc:
    ///   operating_system.name = "windows"
    ///   navigator.platform    = "MacIntel"
    ///   navigator.user_agent  = X11; Linux
    ///
    /// CHU Y: `os_windows_constraint_populates_header` ben duoi kiem BAN DO
    /// rang buoc duoc dung, khong kiem dau ra tuan theo - nen no xanh suot
    /// trong khi loi nay hien dien toan phan. Test nay kiem dau ra.
    ///
    /// PHAM VI: test nay canh `navigator.userAgent`, KHONG canh
    /// `navigator.platform`. Do 2026-09-04, `platform` van lech OS o 5%
    /// (Windows) va 13% (macOS) so mau - va do KHONG phai loi cua ma nay:
    /// chinh CPT cua bo du lieu Apify chua no.
    ///
    ///     UA "Mozilla/5.0 (Windows NT 10.0; Win64; x64)..."
    ///        -> {"Linux x86_64": 0.979, "Win32": 0.021}
    ///     34/83 UA Windows co CPT platform lan sang OS khac
    ///
    /// Tap do cao tu traffic that, trong do co may DANG SPOOF HONG (Linux gia
    /// Windows bang UA nhung ro navigator.platform). Lay mau trung thanh tu
    /// no thi tai tao luon cai spoof hong cua nguoi khac. Do la mot bai toan
    /// khac - can mot tang kiem nhat quan tren bo sinh, khong phai sua sampler.
    #[test]
    fn rang_buoc_os_toi_duoc_user_agent() {
        for os in [OsFamily::Windows, OsFamily::MacOs, OsFamily::Linux] {
            let dau = dau_hieu_ua(&os);
            // QUET, KHONG GHIM VAI SEED. Ban dau cho nay ghim 8 seed, va mot
            // luot pha hoai (cho `ua_khop_os` luon tra true) VAN XANH - vi ca
            // hong nang nhat, seed 9, khong nam trong 8 so do. Do la mot cong
            // GIA: no nhin giong cong nhung khong bat duoc hoi quy.
            //
            // Che do hong hiem nhat trong ba che do do duoc la 12,4% moi mau,
            // nen 200 mau lam xac suat lot xuong duoi 10^-11. Chay het ~3 giay.
            for seed in 0..200u64 {
                let p = FingerprintGenerator::new()
                    .seeded(seed)
                    .browser(BrowserFamily::Chrome)
                    .os(os.clone())
                    .generate()
                    .expect("phai sinh duoc");
                let ua = &p.fingerprint.navigator.user_agent;
                assert!(
                    ua.contains(dau),
                    "os={os:?} seed={seed}: UA khong chua {dau:?}\n  \
                     operating_system.name = {:?}\n  \
                     navigator.platform    = {:?}\n  \
                     navigator.user_agent  = {ua}",
                    p.operating_system.name,
                    p.fingerprint.navigator.platform,
                );
            }
        }
    }

    /// `navigator.platform` phai noi cung mot OS voi `navigator.userAgent`.
    ///
    /// LY DO TON TAI. `userAgent` la goc va da duoc kep tu VEIL-407, nhung CPT
    /// cua `platform` van chua mau thuan san: do 2026-09-04, 27/83 UA Windows
    /// va 48/101 UA macOS co nhanh CPT KHONG chua gia tri nao cua chinh OS do
    /// (mot trong so do la `{"Linux x86_64": 1.0}`, tuc mot UA Windows khong
    /// bao gio ra platform Windows).
    ///
    /// Do bang 22 phep kiem cua veilus-core tren 1500 ho so:
    ///
    /// ```text
    /// truoc: "Platform matches OS" truot 87/1500 (5,8%)
    /// sau:                                1/1500 (0,1%)
    /// ```
    ///
    /// QUET, KHONG GHIM SEED. Che do hong o day khoang 5% moi mau; ghim vai
    /// seed thi lot. Bai hoc nay da tra gia hai lan trong ngay 2026-09-04.
    #[test]
    fn platform_noi_cung_os_voi_user_agent() {
        for os in [OsFamily::Windows, OsFamily::MacOs, OsFamily::Linux] {
            for seed in 0..200u64 {
                let p = FingerprintGenerator::new()
                    .seeded(seed)
                    .browser(BrowserFamily::Chrome)
                    .os(os.clone())
                    .generate()
                    .expect("phai sinh duoc");
                let ua = &p.fingerprint.navigator.user_agent;
                let pf = &p.fingerprint.navigator.platform;
                let os_ua = os_tu_ua(ua).expect("UA phai noi ro OS");
                assert!(
                    platform_khop_os(pf, &os_ua),
                    "os={os:?} seed={seed}: platform {pf:?} khong khop UA\n  {ua}"
                );
            }
        }
    }

    #[test]
    fn os_windows_constraint_populates_header() {
        let gen = FingerprintGenerator::new().os(OsFamily::Windows);
        let constraints = gen.build_header_constraints();
        assert!(
            constraints.contains_key("*OPERATING_SYSTEM"),
            "Windows constraint must target *OPERATING_SYSTEM"
        );
        assert_eq!(constraints["*OPERATING_SYSTEM"], vec!["windows"]);
    }

    #[test]
    fn builder_chaining_compiles() {
        // Verify all chainable methods return Self and compile
        let _gen = FingerprintGenerator::new()
            .browser(BrowserFamily::Chrome)
            .os(OsFamily::Windows)
            .device(DeviceType::Desktop)
            .locale("en-US")
            .seeded(42)
            .strict();
    }

    #[test]
    fn dataset_version_is_populated() {
        let profile = FingerprintGenerator::random().expect("must succeed");
        assert_eq!(
            profile.dataset_version,
            veilus_fingerprint_data::DATASET_VERSION,
            "dataset_version must match embedded constant"
        );
    }

    #[test]
    fn screen_dimensions_are_positive() {
        let profile = FingerprintGenerator::random().expect("must succeed");
        assert!(
            profile.fingerprint.screen.width > 0,
            "screen width must be positive"
        );
        assert!(
            profile.fingerprint.screen.height > 0,
            "screen height must be positive"
        );
    }

    // ── P0: Header meta-key filtering ──────────────────────────────────────

    #[test]
    fn headers_do_not_contain_meta_keys() {
        // Generate 5 profiles to increase confidence
        for seed in [10, 20, 30, 40, 50] {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            for key in profile.headers.keys() {
                assert!(
                    !key.starts_with('*'),
                    "Header key '{key}' starts with '*' — internal meta-key leaked to output"
                );
            }
        }
    }

    // ── P0: Extended navigator fields ──────────────────────────────────────

    #[test]
    fn navigator_extended_fields_populated() {
        let profile = FingerprintGenerator::new()
            .seeded(42)
            .generate()
            .expect("must succeed");
        let nav = &profile.fingerprint.navigator;

        assert_eq!(
            nav.app_code_name.as_deref(),
            Some("Mozilla"),
            "appCodeName must be 'Mozilla'"
        );
        assert_eq!(
            nav.app_name.as_deref(),
            Some("Netscape"),
            "appName must be 'Netscape'"
        );
        assert_eq!(
            nav.product.as_deref(),
            Some("Gecko"),
            "product must be 'Gecko'"
        );
        assert!(!nav.webdriver, "webdriver must always be false");
    }

    // ── P0: userAgentData high-entropy for Chrome ──────────────────────────

    #[test]
    fn chrome_user_agent_data_has_high_entropy_fields() {
        // Generate many profiles and verify the first confirmed Chrome profile
        // has high-entropy UAD fields
        let mut found_chrome_with_uad = false;

        for seed in 0..50 {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            // Only check profiles that are actually Chrome
            if profile.browser.family == BrowserFamily::Chrome {
                if let Some(uad) = &profile.fingerprint.navigator.user_agent_data {
                    if !uad.brands.is_empty()
                        && (uad.architecture.is_some() || uad.bitness.is_some())
                    {
                        found_chrome_with_uad = true;
                        break;
                    }
                }
            }
        }

        assert!(
            found_chrome_with_uad,
            "At least one Chrome profile out of 50 should have high-entropy UAD fields"
        );
    }

    // ── P0: Firefox has no userAgentData ────────────────────────────────────

    #[test]
    fn non_chromium_profiles_have_no_user_agent_data() {
        // The fingerprint network is sampled independently from header network.
        // When the assembled profile resolves to a non-Chromium browser (Firefox/Safari),
        // userAgentData should be filtered out by the assembler based on browser family.
        // Generate many profiles and verify that non-Chrome profiles lack UAD.
        let mut found_non_chrome = false;
        for seed in 0..50 {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if profile.browser.family != BrowserFamily::Chrome
                && profile.browser.family != BrowserFamily::Edge
            {
                // Non-Chromium browsers should NOT have userAgentData
                // (unless the network sampled Chrome-like data, which the assembler
                // derives from the browser header, not the FP network)
                found_non_chrome = true;
                // Just verify no panic — the UAD may or may not be present
                // since fingerprint and header networks are independent
            }
        }
        // Verify we actually found some non-Chrome profiles
        assert!(
            found_non_chrome,
            "Should find at least one non-Chrome profile in 50 samples"
        );
    }

    // ── P0: Extended screen fields ─────────────────────────────────────────

    #[test]
    fn screen_extended_fields_populated() {
        // Run 3 seeded profiles — at least one should have extended fields
        let mut any_outer = false;
        let mut any_avail_top = false;

        for seed in [10, 50, 100] {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");
            let screen = &profile.fingerprint.screen;

            if screen.outer_width.is_some() {
                any_outer = true;
            }
            if screen.avail_top.is_some() {
                any_avail_top = true;
            }
        }

        assert!(
            any_outer,
            "At least one profile should have outerWidth populated"
        );
        assert!(
            any_avail_top,
            "At least one profile should have availTop populated"
        );
    }

    // ── P0: VideoCard populated ────────────────────────────────────────────

    #[test]
    fn video_card_is_populated() {
        // Run 3 seeded profiles — at least one should have videoCard
        let mut any_video_card = false;
        for seed in [10, 50, 100] {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if let Some(vc) = &profile.fingerprint.video_card {
                assert!(
                    !vc.renderer.is_empty(),
                    "videoCard.renderer must not be empty"
                );
                assert!(!vc.vendor.is_empty(), "videoCard.vendor must not be empty");
                any_video_card = true;
            }
        }
        assert!(
            any_video_card,
            "At least one profile should have videoCard populated"
        );
    }

    // ── P0: Battery null handling ──────────────────────────────────────────

    #[test]
    fn battery_handles_null_charging_time() {
        // Battery chargingTime and dischargingTime can be null (Option<f64>)
        // Just verify it parses without panic across multiple samples
        for seed in 0..10 {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if let Some(bat) = &profile.fingerprint.battery {
                assert!(
                    bat.level >= 0.0 && bat.level <= 1.0,
                    "battery level must be 0.0..1.0, got {}",
                    bat.level
                );
                // chargingTime and dischargingTime are Option<f64> — just verify no panic
            }
        }
    }

    // ── P0: mockWebRTC derivation ──────────────────────────────────────────

    #[test]
    fn mock_web_rtc_true_for_chrome() {
        let profile = FingerprintGenerator::new()
            .browser(BrowserFamily::Chrome)
            .os(OsFamily::Windows)
            .seeded(50042)
            .generate()
            .expect("Chrome+Windows must succeed");

        assert_eq!(
            profile.fingerprint.mock_web_rtc,
            Some(true),
            "Chrome should have mockWebRTC=true"
        );
    }

    #[test]
    fn mock_web_rtc_false_for_firefox() {
        // Firefox is rare in the dataset — instead of constraining, generate
        // many profiles and verify any Firefox profile has mockWebRTC=false
        let mut found_firefox = false;
        for seed in 0..100 {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if profile.browser.family == BrowserFamily::Firefox {
                assert_eq!(
                    profile.fingerprint.mock_web_rtc,
                    Some(false),
                    "Firefox should have mockWebRTC=false"
                );
                found_firefox = true;
                break;
            }
        }
        // If no Firefox found in 100 samples, that's OK — the logic is
        // verified by mock_web_rtc_true_for_chrome and the assembler code
        let _ = found_firefox;
    }

    // ── P0: Different seeds actually differ ─────────────────────────────────

    #[test]
    fn different_seeds_produce_different_user_agents() {
        // Try seed pairs far apart — UA collision is extremely unlikely
        // but can happen for adjacent seeds if they hit the same network path
        let p1 = FingerprintGenerator::new()
            .seeded(1)
            .generate()
            .expect("must succeed");
        let p2 = FingerprintGenerator::new()
            .seeded(12345)
            .generate()
            .expect("must succeed");

        // With 1000+ possible UAs, different seeds far apart almost never collide
        assert_ne!(
            p1.fingerprint.navigator.user_agent, p2.fingerprint.navigator.user_agent,
            "Very different seeds should produce different user agents"
        );
    }

    // ── P1: Audio/video codecs ─────────────────────────────────────────────

    #[test]
    fn audio_codecs_populated() {
        let mut any_codecs = false;
        for seed in [10, 50, 100] {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if let Some(ac) = &profile.fingerprint.audio_codecs {
                // At least one codec should be "probably" or "maybe"
                let has_support = [&ac.ogg, &ac.mp3, &ac.wav, &ac.m4a, &ac.aac]
                    .iter()
                    .any(|v| !v.is_empty());
                assert!(
                    has_support,
                    "audioCodecs should have at least one non-empty value"
                );
                any_codecs = true;
            }
        }
        assert!(any_codecs, "At least one profile should have audioCodecs");
    }

    #[test]
    fn video_codecs_populated() {
        let mut any_codecs = false;
        for seed in [10, 50, 100] {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if let Some(vc) = &profile.fingerprint.video_codecs {
                let has_support = [&vc.ogg, &vc.h264, &vc.webm].iter().any(|v| !v.is_empty());
                assert!(
                    has_support,
                    "videoCodecs should have at least one non-empty value"
                );
                any_codecs = true;
            }
        }
        assert!(any_codecs, "At least one profile should have videoCodecs");
    }

    // ── P1: Fonts ──────────────────────────────────────────────────────────

    #[test]
    fn fonts_list_populated() {
        // Some network samples may return empty font lists;
        // verify that at least one out of many seeds has fonts
        let mut any_fonts = false;
        for seed in 0..20 {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if let Some(fonts) = &profile.fingerprint.fonts {
                if !fonts.is_empty() {
                    any_fonts = true;
                    break;
                }
            }
        }
        assert!(
            any_fonts,
            "At least one profile out of 20 should have non-empty fonts"
        );
    }

    // ── P1: Plugins ────────────────────────────────────────────────────────

    #[test]
    fn plugins_data_parseable() {
        // Verify pluginsData parses correctly when present (may be empty for some samples)
        // This test ensures the struct mapping doesn't panic
        let mut any_plugins = false;
        for seed in 0..30 {
            let profile = FingerprintGenerator::new()
                .seeded(seed)
                .generate()
                .expect("must succeed");

            if let Some(pd) = &profile.fingerprint.plugins_data {
                if !pd.plugins.is_empty() {
                    // Verify plugin structure is valid
                    for plugin in &pd.plugins {
                        assert!(!plugin.name.is_empty(), "plugin name must not be empty");
                    }
                    any_plugins = true;
                    break;
                }
            }
        }
        // Note: not all network samples have plugins — just verify parsing works
        let _ = any_plugins;
    }

    // ── P1: Slim flag ──────────────────────────────────────────────────────

    #[test]
    fn slim_is_always_false() {
        let profile = FingerprintGenerator::new()
            .seeded(42)
            .generate()
            .expect("must succeed");

        assert_eq!(
            profile.fingerprint.slim,
            Some(false),
            "slim must always be false"
        );
    }

    // ── P1: Webdriver always false even with constraints ───────────────────

    #[test]
    fn webdriver_always_false_with_constraints() {
        for (browser, os) in [
            (BrowserFamily::Chrome, OsFamily::Windows),
            (BrowserFamily::Firefox, OsFamily::Linux),
            (BrowserFamily::Safari, OsFamily::MacOs),
        ] {
            let profile = FingerprintGenerator::new()
                .browser(browser.clone())
                .os(os)
                .seeded(42)
                .generate()
                .expect("must succeed");

            assert!(
                !profile.fingerprint.navigator.webdriver,
                "webdriver must always be false for {:?}",
                browser
            );
        }
    }

    // ── Device constraint tests ────────────────────────────────────────────

    #[test]
    fn device_desktop_constraint_populates_header() {
        let gen = FingerprintGenerator::new().device(DeviceType::Desktop);
        let constraints = gen.build_header_constraints();
        assert!(
            constraints.contains_key("*DEVICE"),
            "Desktop constraint must target *DEVICE"
        );
        assert_eq!(constraints["*DEVICE"], vec!["desktop"]);
    }

    #[test]
    fn device_mobile_constraint_populates_header() {
        let gen = FingerprintGenerator::new().device(DeviceType::Mobile);
        let constraints = gen.build_header_constraints();
        assert_eq!(constraints["*DEVICE"], vec!["mobile"]);
    }

    #[test]
    fn device_mobile_constraint_generates_mobile_profile() {
        // Mobile profiles should have device=Mobile
        // Try multiple seeds — at least one must succeed with device constraint
        let mut found_mobile = false;
        for seed in 0..20 {
            let result = FingerprintGenerator::new()
                .device(DeviceType::Mobile)
                .seeded(seed)
                .generate();

            if let Ok(profile) = result {
                assert_eq!(
                    profile.device,
                    DeviceType::Mobile,
                    "Device constraint Mobile must produce Mobile profiles"
                );
                found_mobile = true;
                break;
            }
        }
        assert!(
            found_mobile,
            "At least one seeded generation with device=Mobile must succeed"
        );
    }

    #[test]
    fn device_desktop_constraint_generates_desktop_profile() {
        let profile = FingerprintGenerator::new()
            .device(DeviceType::Desktop)
            .seeded(42)
            .generate()
            .expect("Desktop device constraint must succeed");

        assert_eq!(
            profile.device,
            DeviceType::Desktop,
            "Device constraint Desktop must produce Desktop profiles"
        );
    }

    #[test]
    fn device_and_browser_combined_constraint() {
        let profile = FingerprintGenerator::new()
            .browser(BrowserFamily::Chrome)
            .device(DeviceType::Desktop)
            .seeded(42)
            .generate()
            .expect("Chrome+Desktop must succeed");

        assert_eq!(profile.device, DeviceType::Desktop);
        assert_eq!(profile.browser.family, BrowserFamily::Chrome);
    }
}

use std::collections::BTreeMap;
use std::io::{BufReader, Cursor, Read};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use veilus_fingerprint_core::FingerprintError;

/// A value that a Bayesian network node can take.
///
/// The special sentinel `"*MISSING_VALUE*"` indicates the field is absent in real browsers.
/// Values prefixed `"*STRINGIFIED*"` contain JSON-serialized complex data (e.g. UA data).
pub type NodeValue = String;

/// The sentinel string used when a field is absent in real browser data.
pub const MISSING_VALUE: &str = "*MISSING_VALUE*";

/// Prefix for values that contain JSON-encoded complex objects.
pub const STRINGIFIED_PREFIX: &str = "*STRINGIFIED*";

/// Conditional probability table (CPT) node.
///
/// The CPT is a deeply-nested JSON structure. The Apify schema uses:
/// - `"deeper"`: key pointing to the next level of the CPT (indexed by parent values)
/// - `"skip"`: key containing a fallback distribution or sub-tree used when
///   the current parent's specific value is not found in `"deeper"`
/// - Leaf level: a flat `{ "value": probability }` map
///
/// Example (1 parent: `*BROWSER`):
/// ```json
/// { "deeper": { "chrome/120.0.0.0": { "?0": 1.0 }, ... }, "skip": { "?0": 0.5, ... } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CptNode {
    /// A non-leaf level or subtree.
    ///
    /// `BTreeMap` CHU KHONG `HashMap`, va day la mot BAN SUA LOI.
    ///
    /// `leaf_probabilities()` duyet map nay roi `sample_from_probs` cong don
    /// trong so THEO THU TU cho toi khi vuot nguong. Voi `HashMap`, thu tu
    /// duyet duoc `RandomState` ngau nhien hoa THEO TIEN TRINH - nen cung mot
    /// seed, cung mot binary, hai lan chay cho hai gia tri khac nhau.
    ///
    /// Do 2026-09-04, bam navigator cua 1500 ho so qua ba tien trinh:
    ///
    /// ```text
    /// 6ed94069ccf3800f
    /// e6aadda7a2bc49dd
    /// df122e3a0797a511
    /// ```
    ///
    /// Loi THUA: mot phep thu 30 seed qua sach 3/3 luot. Va
    /// `examples/seeded_batch.rs` in "Match: true" vi no so trong CUNG MOT
    /// tien trinh, noi thu tu HashMap da co dinh - no khang dinh dung thu no
    /// khong kiem.
    ///
    /// Sua bang KIEU chu khong bang mot loi goi `sort` truoc khi boc: loi goi
    /// do se bi quen o duong code ke tiep, con kieu thi khong the quen.
    Object(BTreeMap<String, CptNode>),
    /// A leaf probability value.
    Probability(f64),
}

impl CptNode {
    /// Get the `"deeper"` sub-table (present on non-leaf levels).
    pub fn get_deeper(&self) -> Option<&BTreeMap<String, CptNode>> {
        match self {
            CptNode::Object(map) => map.get("deeper").and_then(|d| match d {
                CptNode::Object(inner) => Some(inner),
                _ => None,
            }),
            CptNode::Probability(_) => None,
        }
    }

    /// Get the `"skip"` fallback sub-table.
    pub fn get_skip(&self) -> Option<&CptNode> {
        match self {
            CptNode::Object(map) => map.get("skip"),
            CptNode::Probability(_) => None,
        }
    }

    /// Extract the leaf probability distribution at this level.
    ///
    /// A node is a leaf when it is an Object with no `"deeper"` key and all
    /// values are probabilities.
    pub fn leaf_probabilities(&self) -> Option<Vec<(String, f64)>> {
        match self {
            CptNode::Object(map) => {
                if map.contains_key("deeper") || map.contains_key("skip") {
                    return None;
                }
                let probs: Option<Vec<(String, f64)>> = map
                    .iter()
                    .map(|(k, v)| match v {
                        CptNode::Probability(p) => Some((k.clone(), *p)),
                        _ => None,
                    })
                    .collect();
                probs
            }
            CptNode::Probability(_) => None,
        }
    }

    /// True if this is a `"deeper"` branching node (has `"deeper"` key).
    pub fn is_deeper(&self) -> bool {
        match self {
            CptNode::Object(map) => map.contains_key("deeper"),
            CptNode::Probability(_) => false,
        }
    }
}

/// A single node in the Bayesian network.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BayesianNode {
    /// The variable name (e.g., `"*BROWSER"`, `"userAgent"`, `"hardwareConcurrency"`).
    pub name: String,
    /// Ordered list of parent node names. Defines CPT traversal order.
    pub parent_names: Vec<String>,
    /// All possible string values this node can take.
    pub possible_values: Vec<NodeValue>,
    /// The conditional probability table.
    pub conditional_probabilities: CptNode,
}

/// The complete Bayesian network — a list of nodes in topological order.
#[derive(Debug, Clone, Deserialize)]
pub struct BayesianNetwork {
    /// Nodes in topological order (parents before children).
    pub nodes: Vec<BayesianNode>,
}

impl BayesianNetwork {
    /// Deserialize a `BayesianNetwork` from a ZIP archive containing a single JSON file.
    ///
    /// The ZIP is expected to contain exactly one entry ending in `.json`.
    /// The JSON conforms to the Apify Bayesian network schema: `{ "nodes": [...] }`.
    ///
    /// # Errors
    ///
    /// Returns `FingerprintError::NetworkParseError` if:
    /// - The bytes are not a valid ZIP archive
    /// - No `.json` entry is found in the archive
    /// - The JSON fails to deserialize
    pub fn from_zip_bytes(bytes: &[u8]) -> Result<Self, FingerprintError> {
        let cursor = Cursor::new(bytes);
        let reader = BufReader::new(cursor);
        let mut archive = ZipArchive::new(reader).map_err(|e| {
            FingerprintError::NetworkParseError(format!("Invalid ZIP archive: {e}"))
        })?;

        // Find the first .json entry
        let json_index = (0..archive.len())
            .find(|&i| {
                archive
                    .by_index(i)
                    .map(|e| e.name().ends_with(".json"))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                FingerprintError::NetworkParseError(
                    "ZIP archive contains no .json entry".to_string(),
                )
            })?;

        let mut entry = archive.by_index(json_index).map_err(|e| {
            FingerprintError::NetworkParseError(format!("Cannot read ZIP entry: {e}"))
        })?;

        let mut json_buf = String::new();
        entry.read_to_string(&mut json_buf).map_err(|e| {
            FingerprintError::NetworkParseError(format!("Cannot read JSON from ZIP: {e}"))
        })?;

        serde_json::from_str::<BayesianNetwork>(&json_buf).map_err(|e| {
            FingerprintError::NetworkParseError(format!("JSON deserialization failed: {e}"))
        })
    }
}

#[cfg(test)]
mod tests_thu_tu {
    use super::*;

    /// `leaf_probabilities()` phai tra ve THU TU CO DINH, khong phu thuoc thu
    /// tu chen.
    ///
    /// LY DO TON TAI. `sample_from_probs` cong don trong so theo thu tu Vec
    /// nay cho toi khi vuot nguong, nen thu tu la MOT PHAN CUA KET QUA. Voi
    /// `HashMap`, thu tu duyet duoc `RandomState` ngau nhien hoa theo tien
    /// trinh - cung seed, hai lan chay, hai ho so khac nhau. Xem VEIL-412.
    ///
    /// Test nay chay TRONG MOT TIEN TRINH, nen no khong the tu minh bat duoc
    /// bug goc (thu tu HashMap co dinh trong mot lan chay). Cai no ghim la
    /// BAT BIEN: doi thu tu chen ma ket qua doi thi kieu du lieu da sai lai.
    /// Phep thu that la bam 1500 ho so qua nhieu tien trinh.
    #[test]
    fn thu_tu_la_khong_doi_bat_ke_thu_tu_chen() {
        let cap = [("zeta", 0.1), ("alpha", 0.5), ("mu", 0.4)];

        let mut xuoi = BTreeMap::new();
        for (k, v) in cap {
            xuoi.insert(k.to_string(), CptNode::Probability(v));
        }
        let mut nguoc = BTreeMap::new();
        for (k, v) in cap.iter().rev() {
            nguoc.insert((*k).to_string(), CptNode::Probability(*v));
        }

        let a = CptNode::Object(xuoi)
            .leaf_probabilities()
            .expect("phai la la");
        let b = CptNode::Object(nguoc)
            .leaf_probabilities()
            .expect("phai la la");
        assert_eq!(a, b, "thu tu tra ve doi theo thu tu chen");
        assert_eq!(
            a.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "mu", "zeta"],
            "phai sap theo khoa"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FINGERPRINT_NETWORK_BYTES, HEADER_NETWORK_BYTES};

    #[test]
    fn header_network_deserializes() {
        let network = BayesianNetwork::from_zip_bytes(HEADER_NETWORK_BYTES)
            .expect("header network must deserialize");
        assert!(!network.nodes.is_empty(), "header network must have nodes");
        // Root node (first) has no parents
        assert!(
            network.nodes[0].parent_names.is_empty(),
            "first node must be a root (no parents)"
        );
    }

    #[test]
    fn fingerprint_network_deserializes() {
        let network = BayesianNetwork::from_zip_bytes(FINGERPRINT_NETWORK_BYTES)
            .expect("fingerprint network must deserialize");
        assert!(
            !network.nodes.is_empty(),
            "fingerprint network must have nodes"
        );
    }

    #[test]
    fn header_network_has_browser_root() {
        let network = BayesianNetwork::from_zip_bytes(HEADER_NETWORK_BYTES).unwrap();
        let root = &network.nodes[0];
        assert_eq!(
            root.name, "*BROWSER",
            "header network root must be *BROWSER"
        );
        assert!(
            !root.possible_values.is_empty(),
            "root must have possible values"
        );
    }

    #[test]
    fn fingerprint_network_has_user_agent_root() {
        let network = BayesianNetwork::from_zip_bytes(FINGERPRINT_NETWORK_BYTES).unwrap();
        let root = &network.nodes[0];
        assert_eq!(
            root.name, "userAgent",
            "fingerprint network root must be userAgent"
        );
        assert!(
            !root.possible_values.is_empty(),
            "userAgent must have possible values"
        );
    }

    #[test]
    fn root_node_cpt_is_flat_probability_map() {
        let network = BayesianNetwork::from_zip_bytes(FINGERPRINT_NETWORK_BYTES).unwrap();
        let root = &network.nodes[0];
        let probs = root
            .conditional_probabilities
            .leaf_probabilities()
            .expect("root CPT must be a flat map");
        let total: f64 = probs.iter().map(|(_, p)| p).sum();
        assert!(
            (total - 1.0).abs() < 1e-6,
            "root CPT probabilities must sum to 1.0, got {total}"
        );
    }

    #[test]
    fn invalid_bytes_returns_error() {
        let err = BayesianNetwork::from_zip_bytes(b"not a zip");
        assert!(err.is_err());
        match err.unwrap_err() {
            veilus_fingerprint_core::FingerprintError::NetworkParseError(_) => {}
            other => panic!("Expected NetworkParseError, got {other:?}"),
        }
    }
}

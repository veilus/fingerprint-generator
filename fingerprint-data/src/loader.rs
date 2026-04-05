use std::sync::OnceLock;

use veilus_fingerprint_core::FingerprintError;

use crate::network::BayesianNetwork;
use crate::{FINGERPRINT_NETWORK_BYTES, HEADER_NETWORK_BYTES};

/// Global singleton for the parsed header Bayesian network.
static HEADER_NETWORK: OnceLock<BayesianNetwork> = OnceLock::new();

/// Global singleton for the parsed fingerprint Bayesian network.
static FINGERPRINT_NETWORK: OnceLock<BayesianNetwork> = OnceLock::new();

/// Return the parsed header Bayesian network (parsed on first access, cached thereafter).
///
/// This function is guaranteed to parse the network only once per process lifetime.
/// Subsequent calls return a reference to the same parsed network.
///
/// # Errors
///
/// Returns `FingerprintError::NetworkParseError` if the embedded ZIP bytes fail to parse.
/// This should never happen in a correctly built binary (build.rs validates the ZIPs).
pub fn get_header_network() -> Result<&'static BayesianNetwork, FingerprintError> {
    if let Some(net) = HEADER_NETWORK.get() {
        return Ok(net);
    }
    let parsed = BayesianNetwork::from_zip_bytes(HEADER_NETWORK_BYTES)?;
    // If another thread initialized it before us, we get back our value — either way OK.
    Ok(HEADER_NETWORK.get_or_init(|| parsed))
}

/// Return the parsed fingerprint Bayesian network (parsed on first access, cached thereafter).
///
/// This function is guaranteed to parse the network only once per process lifetime.
/// Subsequent calls return a reference to the same parsed network.
///
/// # Errors
///
/// Returns `FingerprintError::NetworkParseError` if the embedded ZIP bytes fail to parse.
pub fn get_fingerprint_network() -> Result<&'static BayesianNetwork, FingerprintError> {
    if let Some(net) = FINGERPRINT_NETWORK.get() {
        return Ok(net);
    }
    let parsed = BayesianNetwork::from_zip_bytes(FINGERPRINT_NETWORK_BYTES)?;
    Ok(FINGERPRINT_NETWORK.get_or_init(|| parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_network_loads_once() {
        let n1 = get_header_network().expect("must load");
        let n2 = get_header_network().expect("must load again");
        // Same pointer — OnceLock guarantee
        assert!(std::ptr::eq(n1, n2), "must return same singleton reference");
    }

    #[test]
    fn fingerprint_network_loads_once() {
        let n1 = get_fingerprint_network().expect("must load");
        let n2 = get_fingerprint_network().expect("must load again");
        assert!(std::ptr::eq(n1, n2), "must return same singleton reference");
    }

    #[test]
    fn header_network_has_correct_node_count() {
        let net = get_header_network().expect("must load");
        // Apify header network has 19 nodes based on inspection
        assert_eq!(net.nodes.len(), 19, "header network must have 19 nodes");
    }

    #[test]
    fn fingerprint_network_has_correct_node_count() {
        let net = get_fingerprint_network().expect("must load");
        // Apify fingerprint network has 25 nodes based on inspection
        assert_eq!(net.nodes.len(), 25, "fingerprint network must have 25 nodes");
    }
}

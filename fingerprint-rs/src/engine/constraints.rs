use std::collections::HashMap;

use fingerprint_core::FingerprintError;
use fingerprint_data::network::BayesianNetwork;
use rand::Rng;

use super::sampler::sample_ancestral;

/// A constraint map: node name → list of allowed values for that node.
///
/// An empty `Vec<String>` means no restriction (all values allowed for that node).
/// Values use the same encoding as the Bayesian network dataset (e.g., `"chrome/120.0.0.0"`
/// for the `*BROWSER` node).
pub type Constraints = HashMap<String, Vec<String>>;

/// Sample a complete node assignment satisfying all required constraints.
///
/// Uses **rejection sampling with backtracking**: calls [`sample_ancestral`] repeatedly
/// until all constraints are satisfied, or returns `ConstraintsTooRestrictive` after
/// `max_backtracks` unsuccessful attempts.
///
/// # Arguments
///
/// * `network` — The parsed `BayesianNetwork`.
/// * `constraints` — Map of node name → allowed values. Empty vec = no constraint.
/// * `rng` — Any RNG implementing [`rand::Rng`] (random or seeded).
///
/// # Errors
///
/// - `FingerprintError::SamplingFailed` if the underlying sampler fails (network error).
/// - `FingerprintError::ConstraintsTooRestrictive` if constraints cannot be satisfied
///   within `max_backtracks = node_count * 10` attempts.
pub fn sample_constrained(
    network: &BayesianNetwork,
    constraints: &Constraints,
    rng: &mut impl Rng,
) -> Result<HashMap<String, String>, FingerprintError> {
    let node_count = network.nodes.len();
    let max_backtracks = node_count * 10;

    for attempt in 0..=max_backtracks {
        let assignment = sample_ancestral(network, rng)?;

        // Check all constraints — all must be satisfied
        let satisfied = constraints.iter().all(|(node_name, allowed_values)| {
            if allowed_values.is_empty() {
                return true; // No constraint on this node
            }
            assignment
                .get(node_name)
                .map(|v| allowed_values.iter().any(|a| a == v))
                .unwrap_or(false)
        });

        if satisfied {
            tracing::debug!(
                attempt,
                "Constrained sampling succeeded after {} attempt(s)",
                attempt + 1
            );
            return Ok(assignment);
        }

        tracing::debug!(attempt, "Constrained sampling attempt failed, retrying");
    }

    // Build human-readable description of active constraints
    let constraint_desc = constraints
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}=[{}]", v.join(",")))
        .collect::<Vec<_>>()
        .join(", ");

    Err(FingerprintError::ConstraintsTooRestrictive(format!(
        "Could not satisfy constraints after {max_backtracks} attempts: {constraint_desc}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fingerprint_data::loader::{get_fingerprint_network, get_header_network};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn header_browser_prefix_constraint_always_satisfied() {
        // In the header network, *BROWSER values look like "chrome/120.0.0.0".
        // We constrain to a single known value from the dataset.
        let network = get_header_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(12345);

        // Get the actual possible values for *BROWSER
        let browser_node = network
            .nodes
            .iter()
            .find(|n| n.name == "*BROWSER")
            .expect("*BROWSER node must exist");

        // Pick the first possible value
        let first_value = browser_node.possible_values[0].clone();

        let mut constraints = Constraints::new();
        constraints.insert("*BROWSER".to_string(), vec![first_value.clone()]);

        for _ in 0..5 {
            let assignment = sample_constrained(network, &constraints, &mut rng)
                .expect("constraint must be satisfiable");
            assert_eq!(
                assignment.get("*BROWSER").map(|s| s.as_str()),
                Some(first_value.as_str()),
                "*BROWSER constraint must be respected"
            );
        }
    }

    #[test]
    fn fingerprint_ua_constraint_satisfied() {
        let network = get_fingerprint_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(999);

        // Get first userAgent from dataset
        let ua_node = network
            .nodes
            .iter()
            .find(|n| n.name == "userAgent")
            .expect("userAgent node must exist");

        let target_ua = ua_node.possible_values[0].clone();

        let mut constraints = Constraints::new();
        constraints.insert("userAgent".to_string(), vec![target_ua.clone()]);

        let assignment = sample_constrained(network, &constraints, &mut rng)
            .expect("userAgent constraint must be satisfiable");
        assert_eq!(
            assignment.get("userAgent").map(String::as_str),
            Some(target_ua.as_str())
        );
    }

    #[test]
    fn impossible_constraint_returns_error() {
        let network = get_header_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(99999);

        let mut constraints = Constraints::new();
        // "netscape99/1.0" is not a real browser value — no data exists for it
        constraints.insert(
            "*BROWSER".to_string(),
            vec!["netscape99/1.0".to_string()],
        );

        let result = sample_constrained(network, &constraints, &mut rng);
        assert!(
            matches!(
                result,
                Err(FingerprintError::ConstraintsTooRestrictive(_))
            ),
            "Impossible constraint must return ConstraintsTooRestrictive, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn empty_constraints_always_succeeds() {
        let network = get_header_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let constraints = Constraints::new(); // no constraints

        let result = sample_constrained(network, &constraints, &mut rng);
        assert!(result.is_ok(), "Empty constraints must always succeed");
    }
}

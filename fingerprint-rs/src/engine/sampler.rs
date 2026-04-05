use std::collections::HashMap;

use fingerprint_core::FingerprintError;
use fingerprint_data::network::{BayesianNetwork, BayesianNode, CptNode, MISSING_VALUE};
use rand::Rng;

// ─── Topological Sort (Kahn's Algorithm) ────────────────────────────────────

/// Returns node indices in topological order (parents before children).
///
/// Uses Kahn's algorithm. Returns an error if a cycle is detected, which should
/// never happen with the Apify dataset (it is always a DAG).
fn topological_sort(nodes: &[BayesianNode]) -> Result<Vec<usize>, FingerprintError> {
    let name_to_idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();

    let mut in_degree: Vec<usize> = nodes.iter().map(|n| n.parent_names.len()).collect();

    let mut children: Vec<Vec<usize>> = vec![vec![]; nodes.len()];
    for (child_idx, node) in nodes.iter().enumerate() {
        for parent_name in &node.parent_names {
            if let Some(&parent_idx) = name_to_idx.get(parent_name.as_str()) {
                children[parent_idx].push(child_idx);
            }
        }
    }

    let mut queue: std::collections::VecDeque<usize> =
        (0..nodes.len()).filter(|&i| in_degree[i] == 0).collect();

    let mut order = Vec::with_capacity(nodes.len());
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &child in &children[idx] {
            in_degree[child] -= 1;
            if in_degree[child] == 0 {
                queue.push_back(child);
            }
        }
    }

    if order.len() != nodes.len() {
        return Err(FingerprintError::SamplingFailed(
            "Bayesian network has a cycle — cannot perform topological sort".to_string(),
        ));
    }

    Ok(order)
}

// ─── CPT Traversal (Stories 3.1 + 3.2) ─────────────────────────────────────

/// Traverse the CPT recursively given ordered parent values, then sample a value.
///
/// The Apify CPT schema uses:
/// - `"deeper"`: a map from parent value → sub-CPT; navigate by matching `parent_values[0]`
/// - `"skip"`: a fallback distribution or sub-CPT used when the current parent value is
///   not found in `"deeper"`. If present alongside `"deeper"`, it acts as default branch.
/// - Leaf: an Object with no `"deeper"` and no `"skip"` where all values are probabilities.
///
/// Traversal algorithm:
/// 1. If `parent_values` is empty, try to sample from the current leaf.
/// 2. Otherwise, match the first parent value in `deeper`.
///    - If found and the matched sub-node is itself a CPT level (has `deeper` or `skip`),
///      recurse with the sub-node and remaining parents unwrapped out of the `deeper` wrapper.
///    - If found and the matched sub-node is a leaf, sample from it.
///    - If NOT found in `deeper` and a `skip` entry exists, use `skip` as the fallback.
///    - If NOT found and no `skip`, return `SamplingFailed`.
pub(crate) fn traverse_cpt_and_sample(
    cpt: &CptNode,
    parent_values: &[(&str, &str)],
    rng: &mut impl Rng,
) -> Result<String, FingerprintError> {
    // Base case: no more parents to resolve
    if parent_values.is_empty() {
        // Attempt to treat the current node as a leaf
        if let Some(probs) = cpt.leaf_probabilities() {
            return sample_from_probs(&probs, rng);
        }
        // If the node still has a "deeper" (shouldn't happen, but be defensive)
        if let Some(deeper_map) = cpt.get_deeper() {
            // No parent values left but still have deeper — sample aggregating all branches
            // This handles edge cases in the Apify data gracefully
            let all_probs: Vec<(String, f64)> = deeper_map
                .values()
                .filter_map(|v| v.leaf_probabilities())
                .flatten()
                .collect();
            if !all_probs.is_empty() {
                return sample_from_probs(&all_probs, rng);
            }
        }
        return Err(FingerprintError::SamplingFailed(
            "CPT traversal exhausted parents but reached no leaf".to_string(),
        ));
    }

    let (parent_name, parent_value) = parent_values[0];
    let remaining = &parent_values[1..];

    // Try to find the parent value in the "deeper" sub-table
    if let Some(deeper_map) = cpt.get_deeper() {
        if let Some(branch) = deeper_map.get(parent_value) {
            // Found the parent value — recurse with the branch and remaining parents
            return traverse_cpt_and_sample(branch, remaining, rng);
        }
        // Parent value not in deeper — check for skip fallback
        if let Some(skip) = cpt.get_skip() {
            return traverse_cpt_and_sample(skip, remaining, rng);
        }
        // No skip either — sampling failed
        return Err(FingerprintError::SamplingFailed(format!(
            "CPT has no entry and no skip for parent '{parent_name}' = '{parent_value}'"
        )));
    }

    // No "deeper" key at this level — this IS the leaf; sample it
    if let Some(probs) = cpt.leaf_probabilities() {
        return sample_from_probs(&probs, rng);
    }

    Err(FingerprintError::SamplingFailed(format!(
        "CPT node for parent '{parent_name}' is neither a leaf nor has 'deeper'"
    )))
}

/// Sample a value from a probability distribution using weighted random selection.
///
/// Handles floating-point precision edge cases by falling back to the last entry.
/// Skips the `*MISSING_VALUE*` sentinel — it represents a field that is absent in
/// the real browser and should not be emitted as a sampled value.
///
/// Wait — actually `*MISSING_VALUE*` IS a valid sample output; the calling code
/// (Story 4.4 assembler) decides how to handle it. We sample it faithfully.
pub(crate) fn sample_from_probs(
    probs: &[(String, f64)],
    rng: &mut impl Rng,
) -> Result<String, FingerprintError> {
    if probs.is_empty() {
        return Err(FingerprintError::SamplingFailed(
            "Cannot sample from empty probability distribution".to_string(),
        ));
    }

    let total: f64 = probs.iter().map(|(_, p)| p).sum();
    if total <= 0.0 {
        return Err(FingerprintError::SamplingFailed(
            "Probability distribution has zero total weight".to_string(),
        ));
    }

    let mut threshold: f64 = rng.gen::<f64>() * total;
    for (value, prob) in probs {
        threshold -= prob;
        if threshold <= 0.0 {
            return Ok(value.clone());
        }
    }

    // Floating-point precision fallback: return the last entry
    Ok(probs.last().unwrap().0.clone())
}

// ─── Ancestral Sampler (Story 3.1) ──────────────────────────────────────────

/// Sample a complete variable assignment for all nodes in the Bayesian network.
///
/// Performs **ancestral sampling**: iterates nodes in topological order (parents before
/// children) and for each node samples a value conditioned on its already-sampled parents.
///
/// # Arguments
///
/// * `network` — The parsed `BayesianNetwork` from [`fingerprint_data::loader`].
/// * `rng` — Any RNG implementing [`rand::Rng`] (random or seeded).
///
/// # Returns
///
/// A `HashMap<node_name, sampled_value>` with one entry per node. The sampled values
/// include the `*MISSING_VALUE*` sentinel where real browsers omit a field.
///
/// # Errors
///
/// Returns `FingerprintError::SamplingFailed` if:
/// - The network has a cycle (defensive check — Apify data is always acyclic)
/// - A CPT branch is missing for this parent combination
pub fn sample_ancestral(
    network: &BayesianNetwork,
    rng: &mut impl Rng,
) -> Result<HashMap<String, String>, FingerprintError> {
    let nodes = &network.nodes;
    let order = topological_sort(nodes)?;
    let mut assignment: HashMap<String, String> = HashMap::with_capacity(nodes.len());

    for idx in order {
        let node = &nodes[idx];
        // Build ordered parent context: [(parent_name, sampled_value), ...]
        // Order MUST match parent_names array to align with CPT nesting depth.
        let parent_values: Vec<(&str, &str)> = node
            .parent_names
            .iter()
            .filter_map(|pname| {
                assignment
                    .get(pname.as_str())
                    .map(|v| (pname.as_str(), v.as_str()))
            })
            .collect();

        let value =
            traverse_cpt_and_sample(&node.conditional_probabilities, &parent_values, rng)?;
        assignment.insert(node.name.clone(), value);
    }

    Ok(assignment)
}

/// Return the MISSING_VALUE sentinel for convenience.
pub fn missing_value() -> &'static str {
    MISSING_VALUE
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use fingerprint_data::loader::{get_fingerprint_network, get_header_network};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn ancestral_sample_has_all_nodes_header() {
        let network = get_header_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let assignment = sample_ancestral(network, &mut rng).expect("must sample");
        assert_eq!(
            assignment.len(),
            network.nodes.len(),
            "assignment must have one entry per node"
        );
    }

    #[test]
    fn ancestral_sample_has_all_nodes_fingerprint() {
        let network = get_fingerprint_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(2);
        let assignment = sample_ancestral(network, &mut rng).expect("must sample");
        assert_eq!(assignment.len(), network.nodes.len());
    }

    #[test]
    fn header_sample_is_deterministic_for_seed() {
        let network = get_header_network().expect("must load");
        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);
        let a1 = sample_ancestral(network, &mut rng1).expect("must sample");
        let a2 = sample_ancestral(network, &mut rng2).expect("must sample");
        assert_eq!(a1, a2, "same seed must yield identical output");
    }

    #[test]
    fn different_seeds_produce_different_samples() {
        let network = get_header_network().expect("must load");
        let mut rng1 = ChaCha8Rng::seed_from_u64(1);
        let mut rng2 = ChaCha8Rng::seed_from_u64(9999);
        let a1 = sample_ancestral(network, &mut rng1).expect("must sample");
        let a2 = sample_ancestral(network, &mut rng2).expect("must sample");
        // Very unlikely to be identical with distinct seeds
        assert_ne!(a1, a2, "different seeds should produce different samples");
    }

    #[test]
    fn browser_has_chrome_distribution() {
        let network = get_header_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let total = 500usize;
        let mut chrome_count = 0usize;

        for _ in 0..total {
            let assignment = sample_ancestral(network, &mut rng).expect("must sample");
            if let Some(browser) = assignment.get("*BROWSER") {
                if browser.starts_with("chrome") {
                    chrome_count += 1;
                }
            }
        }

        let ratio = chrome_count as f64 / total as f64;
        assert!(
            (0.50..=0.85).contains(&ratio),
            "Chrome ratio {ratio:.2} must be in [0.50, 0.85] — actual Apify distribution"
        );
    }

    #[test]
    fn topological_order_respected() {
        let network = get_header_network().expect("must load");
        let order = topological_sort(&network.nodes).expect("must sort");

        let name_to_pos: HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(pos, &idx)| (network.nodes[idx].name.as_str(), pos))
            .collect();

        for node in &network.nodes {
            let node_pos = name_to_pos[node.name.as_str()];
            for parent_name in &node.parent_names {
                let parent_pos = name_to_pos
                    .get(parent_name.as_str())
                    .copied()
                    .unwrap_or(usize::MAX);
                assert!(
                    parent_pos < node_pos,
                    "Parent '{}' must appear before child '{}' in topological order",
                    parent_name,
                    node.name
                );
            }
        }
    }

    #[test]
    fn fingerprint_network_has_user_agent_field() {
        let network = get_fingerprint_network().expect("must load");
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let assignment = sample_ancestral(network, &mut rng).expect("must sample");
        assert!(
            assignment.contains_key("userAgent"),
            "fingerprint assignment must contain userAgent"
        );
    }
}

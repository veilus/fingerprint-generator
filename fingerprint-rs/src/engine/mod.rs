/// Ancestral sampler for Bayesian network traversal.
///
/// Exposes [`sample_ancestral`] and [`sample_constrained`].
pub mod sampler;

/// Constraint-based rejection sampler.
pub mod constraints;

pub use constraints::{sample_constrained, Constraints};
pub use sampler::{sample_ancestral, sample_ancestral_with_evidence};

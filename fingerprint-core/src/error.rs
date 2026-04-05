/// All errors that can be produced by fingerprint-rs.
///
/// Implemented fully in Story 1.4.
#[derive(thiserror::Error, Debug)]
pub enum FingerprintError {
    /// Failure to parse or deserialize the embedded Bayesian network ZIP.
    #[error("Failed to parse network definition: {0}")]
    NetworkParseError(String),

    /// The requested browser/OS combination is mutually exclusive.
    #[error("Constraint conflict: browser '{browser}' is not supported on os '{os}'")]
    ConstraintConflict {
        /// The browser family constraint.
        browser: String,
        /// The OS family constraint.
        os: String,
    },

    /// The sampler could not find a valid assignment satisfying all constraints.
    #[error("Constraints too restrictive: {0}")]
    ConstraintsTooRestrictive(String),

    /// A field value in GenerateRequest is not recognized by the library.
    #[error("Invalid constraint: field '{field}' has unrecognized value '{value}'")]
    InvalidConstraint {
        /// The field name.
        field: String,
        /// The unrecognized value.
        value: String,
    },

    /// The sampling algorithm failed unexpectedly.
    #[error("Sampling failed: {0}")]
    SamplingFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_conflict_display() {
        let err = FingerprintError::ConstraintConflict {
            browser: "safari".into(),
            os: "windows".into(),
        };
        assert_eq!(
            err.to_string(),
            "Constraint conflict: browser 'safari' is not supported on os 'windows'"
        );
    }

    #[test]
    fn invalid_constraint_display() {
        let err = FingerprintError::InvalidConstraint {
            field: "browser".into(),
            value: "netscape".into(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid constraint: field 'browser' has unrecognized value 'netscape'"
        );
    }

    #[test]
    fn constraints_too_restrictive_display() {
        let err = FingerprintError::ConstraintsTooRestrictive("no chrome on ios data".into());
        assert_eq!(
            err.to_string(),
            "Constraints too restrictive: no chrome on ios data"
        );
    }

    #[test]
    fn sampling_failed_display() {
        let err = FingerprintError::SamplingFailed("CPT branch not found".into());
        assert_eq!(err.to_string(), "Sampling failed: CPT branch not found");
    }

    #[test]
    fn network_parse_error_display() {
        let err = FingerprintError::NetworkParseError("invalid zip format".into());
        assert_eq!(
            err.to_string(),
            "Failed to parse network definition: invalid zip format"
        );
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(FingerprintError::SamplingFailed("test".into()));
        assert!(!err.to_string().is_empty());
    }
}

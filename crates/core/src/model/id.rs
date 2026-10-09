//! Stable identifiers (D-022).
//!
//! Requests and environments carry a UUID v4 that is generated once, when the
//! item is created, and never changes. Local state (literal values, secrets)
//! is keyed by it, so it keeps working when a teammate renames or moves the file.

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

/// An id string that is not a lowercase, hyphenated UUID.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid id (expected a lowercase, hyphenated UUID)")]
pub struct IdParseError(pub String);

fn parse_canonical(s: &str) -> Result<Uuid, IdParseError> {
    match Uuid::try_parse(s) {
        // Only one spelling is accepted, so the same id is always the same bytes on disk.
        Ok(uuid) if uuid.as_hyphenated().to_string() == s => Ok(uuid),
        _ => Err(IdParseError(s.to_owned())),
    }
}

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Generates a new random id. Call this only when the item is created.
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0.as_hyphenated())
            }
        }

        impl FromStr for $name {
            type Err = IdParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                parse_canonical(s).map(Self)
            }
        }
    };
}

id_type!(
    /// Identifies a request across renames and moves.
    RequestId
);
id_type!(
    /// Identifies an environment across renames.
    EnvironmentId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_round_trip_through_text() {
        let id = RequestId::generate();
        let text = id.to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(text.parse::<RequestId>(), Ok(id));
    }

    #[test]
    fn generated_ids_differ() {
        assert_ne!(EnvironmentId::generate(), EnvironmentId::generate());
    }

    #[test]
    fn only_the_canonical_spelling_is_accepted() {
        let canonical = "3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70";
        assert!(canonical.parse::<RequestId>().is_ok());
        for bad in [
            "",
            "not-an-id",
            "3F2B8C1E-7A4D-4E9B-9C55-0D6A1B2E4F70",
            "3f2b8c1e7a4d4e9b9c550d6a1b2e4f70",
            "{3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70}",
            "urn:uuid:3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70",
            " 3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70",
        ] {
            assert_eq!(bad.parse::<RequestId>(), Err(IdParseError(bad.to_owned())));
        }
    }
}

//! Sibling ordering keys (D-007).

use std::fmt;

/// A fractional-index key that positions an item among its siblings.
///
/// Siblings sort by key (plain byte order), then by file name. Generating a
/// key between two others, and renormalizing a group, is the workspace
/// store's job; this type only guarantees the key is well formed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OrderKey(String);

/// An order key that is empty or contains characters other than `0-9 A-Z a-z`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid order key (expected one or more of 0-9, A-Z, a-z)")]
pub struct OrderKeyError(pub String);

impl OrderKey {
    /// Wraps `key` after checking it is non-empty ASCII alphanumeric.
    pub fn new(key: impl Into<String>) -> Result<Self, OrderKeyError> {
        let key = key.into();
        if !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric()) {
            Ok(Self(key))
        } else {
            Err(OrderKeyError(key))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrderKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_keys_are_accepted_and_sort_by_bytes() {
        let mut keys: Vec<OrderKey> = ["a1", "Zz", "a0", "a0V", "b"]
            .into_iter()
            .map(|k| OrderKey::new(k).unwrap())
            .collect();
        keys.sort();
        let sorted: Vec<&str> = keys.iter().map(OrderKey::as_str).collect();
        assert_eq!(sorted, ["Zz", "a0", "a0V", "a1", "b"]);
    }

    #[test]
    fn invalid_keys_are_rejected() {
        for bad in ["", " ", "a-1", "a.1", "é", "a 1"] {
            assert_eq!(OrderKey::new(bad), Err(OrderKeyError(bad.to_owned())));
        }
    }
}

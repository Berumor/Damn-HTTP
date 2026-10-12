//! Sibling ordering keys (D-007).

use std::fmt;

/// A fractional-index key that positions an item among its siblings.
///
/// Siblings sort by key (plain byte order), then by file name.
///
/// Any non-empty string of `0-9 A-Z a-z` is accepted, so a key written by
/// hand still loads. Keys the app generates ([`OrderKey::first`],
/// [`OrderKey::between`]) follow the usual fractional-indexing layout: an
/// integer part whose first character gives its length (`a0`, `a1`, ...,
/// `az`, `b00`, ...), then an optional fraction (`a0V`).
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

    /// The key given to the first item of an empty group.
    pub fn first() -> Self {
        Self("a0".to_owned())
    }

    /// A key that sorts after `before` and before `after`. `None` means
    /// "no neighbour on that side".
    ///
    /// Returns `None` when no such key can be generated: the neighbours are
    /// equal or out of order, or one of them does not follow the layout the
    /// app generates (a key edited by hand). The caller then renormalizes
    /// the whole group with [`OrderKey::sequence`].
    pub fn between(before: Option<&OrderKey>, after: Option<&OrderKey>) -> Option<Self> {
        let a = before.map(|k| k.0.as_bytes());
        let b = after.map(|k| k.0.as_bytes());
        if let Some(a) = a {
            split_key(a)?;
        }
        if let Some(b) = b {
            split_key(b)?;
        }
        if let (Some(a), Some(b)) = (a, b)
            && a >= b
        {
            return None;
        }
        let key = generate_between(a, b)?;
        String::from_utf8(key).ok().map(Self)
    }

    /// `count` short keys in ascending order, used to renormalize a group.
    pub fn sequence(count: usize) -> Vec<Self> {
        let mut keys: Vec<Self> = Vec::with_capacity(count);
        for _ in 0..count {
            let next = match keys.last() {
                None => Some(Self::first()),
                Some(last) => Self::between(Some(last), None),
            };
            // Appending never fails before `z` followed by 26 `z` digits,
            // far beyond any real group; stop there instead of panicking.
            match next {
                Some(key) => keys.push(key),
                None => break,
            }
        }
        keys
    }
}

const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn digit_value(byte: u8) -> Option<usize> {
    DIGITS.iter().position(|d| *d == byte)
}

/// Length of the integer part announced by its first character.
fn integer_length(head: u8) -> Option<usize> {
    match head {
        b'a'..=b'z' => Some(usize::from(head - b'a') + 2),
        b'A'..=b'Z' => Some(usize::from(b'Z' - head) + 2),
        _ => None,
    }
}

/// Splits a key into integer part and fraction, or `None` if it does not
/// follow the generated layout.
fn split_key(key: &[u8]) -> Option<(&[u8], &[u8])> {
    let length = integer_length(*key.first()?)?;
    if key.len() < length {
        return None;
    }
    let (integer, fraction) = key.split_at(length);
    // A fraction never ends in the zero digit: nothing sorts between `x` and `x0`.
    if fraction.last() == Some(&DIGITS[0]) {
        return None;
    }
    // The smallest integer has nothing below it, so it is not generated.
    if integer[0] == b'A' && integer[1..].iter().all(|d| *d == DIGITS[0]) && fraction.is_empty() {
        return None;
    }
    Some((integer, fraction))
}

/// A fraction strictly between `a` and `b`, where `None` for `b` means 1.
fn midpoint(a: &[u8], b: Option<&[u8]>) -> Option<Vec<u8>> {
    if let Some(b) = b {
        let mut common = 0;
        while common < b.len() && a.get(common).copied().unwrap_or(DIGITS[0]) == b[common] {
            common += 1;
        }
        if common > 0 {
            let rest_a = a.get(common..).unwrap_or(&[]);
            let mut result = b[..common].to_vec();
            result.extend(midpoint(rest_a, Some(&b[common..]))?);
            return Some(result);
        }
    }
    let digit_a = match a.first() {
        Some(byte) => digit_value(*byte)?,
        None => 0,
    };
    let digit_b = match b.and_then(|b| b.first()) {
        Some(byte) => digit_value(*byte)?,
        None => DIGITS.len(),
    };
    if digit_b > digit_a + 1 {
        let middle = (digit_a + digit_b).div_ceil(2);
        return Some(vec![DIGITS[middle]]);
    }
    match b {
        Some(b) if b.len() > 1 => Some(b[..1].to_vec()),
        _ => {
            let mut result = vec![DIGITS[digit_a]];
            result.extend(midpoint(a.get(1..).unwrap_or(&[]), None)?);
            Some(result)
        }
    }
}

fn increment_integer(integer: &[u8]) -> Option<Vec<u8>> {
    let (head, digits) = integer.split_first()?;
    let mut digits = digits.to_vec();
    let mut carry = true;
    for digit in digits.iter_mut().rev() {
        let value = digit_value(*digit)? + 1;
        if value == DIGITS.len() {
            *digit = DIGITS[0];
        } else {
            *digit = DIGITS[value];
            carry = false;
            break;
        }
    }
    let mut head = *head;
    if carry {
        if head == b'Z' {
            return Some(b"a0".to_vec());
        }
        if head == b'z' {
            return None;
        }
        head += 1;
        if head > b'a' {
            digits.push(DIGITS[0]);
        } else {
            digits.pop();
        }
    }
    let mut result = vec![head];
    result.extend(digits);
    Some(result)
}

fn decrement_integer(integer: &[u8]) -> Option<Vec<u8>> {
    let (head, digits) = integer.split_first()?;
    let mut digits = digits.to_vec();
    let mut borrow = true;
    for digit in digits.iter_mut().rev() {
        let value = digit_value(*digit)?;
        if value == 0 {
            *digit = DIGITS[DIGITS.len() - 1];
        } else {
            *digit = DIGITS[value - 1];
            borrow = false;
            break;
        }
    }
    let mut head = *head;
    if borrow {
        if head == b'a' {
            return Some(b"Zz".to_vec());
        }
        if head == b'A' {
            return None;
        }
        head -= 1;
        if head < b'Z' {
            digits.push(DIGITS[DIGITS.len() - 1]);
        } else {
            digits.pop();
        }
    }
    let mut result = vec![head];
    result.extend(digits);
    Some(result)
}

fn generate_between(a: Option<&[u8]>, b: Option<&[u8]>) -> Option<Vec<u8>> {
    match (a, b) {
        (None, None) => Some(b"a0".to_vec()),
        (None, Some(b)) => {
            let (integer, fraction) = split_key(b)?;
            if !fraction.is_empty() {
                return Some(integer.to_vec());
            }
            match decrement_integer(integer) {
                // The smallest integer is not a usable key; go fractional below it.
                Some(lower) if split_key(&lower).is_some() => Some(lower),
                Some(lower) => {
                    let mut result = lower;
                    result.extend(midpoint(&[], None)?);
                    Some(result)
                }
                None => None,
            }
        }
        (Some(a), None) => {
            let (integer, fraction) = split_key(a)?;
            match increment_integer(integer) {
                Some(next) => Some(next),
                None => {
                    let mut result = integer.to_vec();
                    result.extend(midpoint(fraction, None)?);
                    Some(result)
                }
            }
        }
        (Some(a), Some(b)) => {
            let (integer_a, fraction_a) = split_key(a)?;
            let (integer_b, fraction_b) = split_key(b)?;
            if integer_a == integer_b {
                let mut result = integer_a.to_vec();
                result.extend(midpoint(fraction_a, Some(fraction_b))?);
                return Some(result);
            }
            if let Some(next) = increment_integer(integer_a)
                && next.as_slice() < b
            {
                return Some(next);
            }
            let mut result = integer_a.to_vec();
            result.extend(midpoint(fraction_a, None)?);
            Some(result)
        }
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

    fn key(text: &str) -> OrderKey {
        OrderKey::new(text).unwrap()
    }

    fn between(before: Option<&str>, after: Option<&str>) -> Option<String> {
        let before = before.map(key);
        let after = after.map(key);
        OrderKey::between(before.as_ref(), after.as_ref()).map(|k| k.as_str().to_owned())
    }

    #[test]
    fn known_results() {
        assert_eq!(between(None, None).as_deref(), Some("a0"));
        assert_eq!(between(Some("a0"), None).as_deref(), Some("a1"));
        assert_eq!(between(Some("az"), None).as_deref(), Some("b00"));
        assert_eq!(between(None, Some("a0")).as_deref(), Some("Zz"));
        assert_eq!(between(Some("a0"), Some("a1")).as_deref(), Some("a0V"));
        assert_eq!(between(Some("a0"), Some("a0V")).as_deref(), Some("a0G"));
        assert_eq!(between(Some("a0"), Some("a2")).as_deref(), Some("a1"));
        assert_eq!(between(None, Some("a0V")).as_deref(), Some("a0"));
        assert_eq!(between(Some("Zz"), Some("a1")).as_deref(), Some("a0"));
    }

    #[test]
    fn impossible_requests_give_none() {
        assert_eq!(between(Some("a1"), Some("a0")), None, "out of order");
        assert_eq!(between(Some("a1"), Some("a1")), None, "equal");
        assert_eq!(between(Some("1"), None), None, "hand-written key");
        assert_eq!(
            between(Some("b"), Some("c")),
            None,
            "integer part too short"
        );
        assert_eq!(
            between(Some("a0"), Some("a00")),
            None,
            "fraction ending in zero"
        );
    }

    #[test]
    fn sequence_is_ascending_and_short() {
        let keys = OrderKey::sequence(200);
        assert_eq!(keys.len(), 200);
        assert_eq!(keys[0].as_str(), "a0");
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(keys.iter().all(|k| k.as_str().len() <= 3));
    }

    /// Inserts thousands of keys at pseudo-random positions and checks every
    /// result is valid and lands strictly between its neighbours.
    #[test]
    fn inserting_anywhere_keeps_the_order() {
        let mut keys = vec![OrderKey::first()];
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..5000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let position = usize::try_from(state % (keys.len() as u64 + 1)).unwrap();
            let before = position.checked_sub(1).map(|i| &keys[i]);
            let after = keys.get(position);
            let new = OrderKey::between(before, after).unwrap();
            assert!(OrderKey::new(new.as_str()).is_ok());
            assert!(before.is_none_or(|b| *b < new), "{before:?} < {new:?}");
            assert!(after.is_none_or(|a| new < *a), "{new:?} < {after:?}");
            keys.insert(position, new);
        }
    }

    /// Always inserting at the front, or always just after the first item,
    /// are the patterns that make keys grow fastest.
    #[test]
    fn repeated_inserts_at_one_end_stay_valid() {
        let mut first = OrderKey::first();
        for _ in 0..2000 {
            let new = OrderKey::between(None, Some(&first)).unwrap();
            assert!(new < first);
            first = new;
        }
        let low = OrderKey::first();
        let mut high = OrderKey::between(Some(&low), None).unwrap();
        for _ in 0..500 {
            let new = OrderKey::between(Some(&low), Some(&high)).unwrap();
            assert!(low < new && new < high);
            high = new;
        }
    }

    #[test]
    fn invalid_keys_are_rejected() {
        for bad in ["", " ", "a-1", "a.1", "é", "a 1"] {
            assert_eq!(OrderKey::new(bad), Err(OrderKeyError(bad.to_owned())));
        }
    }
}

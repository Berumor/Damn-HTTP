//! Typed, strict access to a parsed file.
//!
//! Scalars are read by what the key expects, not by YAML's own type
//! resolution: a string key takes the text of any scalar (`value: 8080` is
//! the string `8080`, `name: no` is the string `no`), while a boolean key
//! takes only plain `true` or `false`, and a number key only plain digits.
//! Every key must be known (D-009).

use super::error::{FormatError, FormatErrorKind};
use super::tree::{Entry, Node, NodeKind};

pub(crate) struct MapReader<'n> {
    node: &'n Node,
    entries: &'n [Entry],
    used: Vec<bool>,
    path: String,
}

impl<'n> MapReader<'n> {
    pub(crate) fn new(node: &'n Node, path: impl Into<String>) -> Result<Self, FormatError> {
        let path = path.into();
        match &node.kind {
            NodeKind::Map(entries) => Ok(Self {
                node,
                entries,
                used: vec![false; entries.len()],
                path,
            }),
            _ => Err(node.error(FormatErrorKind::WrongType {
                path,
                expected: "a mapping",
            })),
        }
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn child_path(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    /// The node under `key`, marking the key as known. A null value counts
    /// as absent.
    pub(crate) fn node(&mut self, key: &str) -> Option<&'n Node> {
        let index = self.entries.iter().position(|e| e.key == key)?;
        self.used[index] = true;
        let node = &self.entries[index].value;
        (!node.is_null()).then_some(node)
    }

    fn missing(&self, key: &str) -> FormatError {
        self.node.error(FormatErrorKind::MissingKey {
            path: self.path.clone(),
            key: key.to_owned(),
        })
    }

    pub(crate) fn required(&mut self, key: &str) -> Result<&'n Node, FormatError> {
        self.node(key).ok_or_else(|| self.missing(key))
    }

    /// A string, or `""` when the key is absent.
    pub(crate) fn string(&mut self, key: &str) -> Result<String, FormatError> {
        match self.node(key) {
            Some(node) => scalar_string(node, &self.child_path(key)),
            None => Ok(String::new()),
        }
    }

    pub(crate) fn optional_string(&mut self, key: &str) -> Result<Option<String>, FormatError> {
        self.node(key)
            .map(|node| scalar_string(node, &self.child_path(key)))
            .transpose()
    }

    pub(crate) fn required_string(&mut self, key: &str) -> Result<String, FormatError> {
        let node = self.required(key)?;
        scalar_string(node, &self.child_path(key))
    }

    /// A required string turned into a typed value; `problem` describes a
    /// value `parse` rejects.
    pub(crate) fn required_parsed<T>(
        &mut self,
        key: &str,
        problem: &'static str,
        parse: impl FnOnce(&str) -> Option<T>,
    ) -> Result<T, FormatError> {
        let node = self.required(key)?;
        let path = self.child_path(key);
        let text = scalar_string(node, &path)?;
        parse(&text).ok_or_else(|| node.error(FormatErrorKind::InvalidValue { path, problem }))
    }

    pub(crate) fn bool_or(&mut self, key: &str, default: bool) -> Result<bool, FormatError> {
        match self.node(key) {
            Some(node) => scalar_bool(node, &self.child_path(key)),
            None => Ok(default),
        }
    }

    pub(crate) fn optional_bool(&mut self, key: &str) -> Result<Option<bool>, FormatError> {
        self.node(key)
            .map(|node| scalar_bool(node, &self.child_path(key)))
            .transpose()
    }

    pub(crate) fn optional_u64(&mut self, key: &str) -> Result<Option<u64>, FormatError> {
        self.node(key)
            .map(|node| scalar_u64(node, &self.child_path(key)))
            .transpose()
    }

    pub(crate) fn optional_u32(&mut self, key: &str) -> Result<Option<u32>, FormatError> {
        let Some(node) = self.node(key) else {
            return Ok(None);
        };
        let path = self.child_path(key);
        let n = scalar_u64(node, &path)?;
        u32::try_from(n).map(Some).map_err(|_| {
            node.error(FormatErrorKind::InvalidValue {
                path,
                problem: "the number is too large",
            })
        })
    }

    /// The items of a list, or none when the key is absent.
    pub(crate) fn list(&mut self, key: &str) -> Result<&'n [Node], FormatError> {
        match self.node(key) {
            None => Ok(&[]),
            Some(Node {
                kind: NodeKind::List(items),
                ..
            }) => Ok(items),
            Some(node) => Err(node.error(FormatErrorKind::WrongType {
                path: self.child_path(key),
                expected: "a list",
            })),
        }
    }

    pub(crate) fn optional_map(&mut self, key: &str) -> Result<Option<MapReader<'n>>, FormatError> {
        let path = self.child_path(key);
        self.node(key)
            .map(|node| MapReader::new(node, path))
            .transpose()
    }

    /// Fails on the first key that was never asked for.
    pub(crate) fn finish(self) -> Result<(), FormatError> {
        match self.used.iter().position(|used| !used) {
            None => Ok(()),
            Some(index) => {
                let entry = &self.entries[index];
                Err(entry.error(FormatErrorKind::UnknownKey {
                    path: self.path,
                    key: entry.key.clone(),
                }))
            }
        }
    }

    /// The remaining entries, for mappings whose keys are data (ids, names).
    pub(crate) fn entries(self) -> &'n [Entry] {
        self.entries
    }
}

/// `path[index]`, for list items.
pub(crate) fn item_path(path: &str, index: usize) -> String {
    format!("{path}[{index}]")
}

pub(crate) fn scalar_string(node: &Node, path: &str) -> Result<String, FormatError> {
    match &node.kind {
        _ if node.is_null() => Ok(String::new()),
        NodeKind::Scalar { value, .. } => Ok(value.clone()),
        _ => Err(node.error(FormatErrorKind::WrongType {
            path: path.to_owned(),
            expected: "a string",
        })),
    }
}

fn scalar_bool(node: &Node, path: &str) -> Result<bool, FormatError> {
    let text = match &node.kind {
        NodeKind::Scalar { value, plain: true } => value.as_str(),
        _ => "",
    };
    match text {
        "true" | "True" | "TRUE" => Ok(true),
        "false" | "False" | "FALSE" => Ok(false),
        _ => Err(node.error(FormatErrorKind::WrongType {
            path: path.to_owned(),
            expected: "true or false",
        })),
    }
}

fn scalar_u64(node: &Node, path: &str) -> Result<u64, FormatError> {
    match &node.kind {
        NodeKind::Scalar { value, plain: true }
            if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) =>
        {
            value.parse().ok()
        }
        _ => None,
    }
    .ok_or_else(|| {
        node.error(FormatErrorKind::WrongType {
            path: path.to_owned(),
            expected: "a whole number",
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::tree::parse;

    #[test]
    fn strings_take_any_scalar_text() {
        let root = parse("a: 8080\nb: no\nc: '1.0'\nd:\ne: ~\n").unwrap();
        let mut map = MapReader::new(&root, "").unwrap();
        assert_eq!(map.string("a").unwrap(), "8080");
        assert_eq!(map.string("b").unwrap(), "no");
        assert_eq!(map.string("c").unwrap(), "1.0");
        assert_eq!(map.optional_string("d").unwrap(), None);
        assert_eq!(map.string("e").unwrap(), "");
        assert_eq!(map.string("absent").unwrap(), "");
        map.finish().unwrap();
    }

    #[test]
    fn booleans_and_numbers_are_strict() {
        let root = parse("a: true\nb: 'true'\nc: yes\nd: 30\ne: -1\nf: 1.5\ng: \"3\"\n").unwrap();
        let mut map = MapReader::new(&root, "s").unwrap();
        assert_eq!(map.bool_or("a", false), Ok(true));
        for key in ["b", "c"] {
            assert_eq!(
                map.bool_or(key, false).unwrap_err().kind,
                FormatErrorKind::WrongType {
                    path: format!("s.{key}"),
                    expected: "true or false"
                }
            );
        }
        assert_eq!(map.optional_u64("d"), Ok(Some(30)));
        for key in ["e", "f", "g"] {
            assert!(map.optional_u64(key).is_err(), "{key}");
        }
    }

    #[test]
    fn unknown_keys_are_reported_where_they_are() {
        let root = parse("name: x\nnmae: y\n").unwrap();
        let mut map = MapReader::new(&root, "").unwrap();
        map.string("name").unwrap();
        let err = map.finish().unwrap_err();
        assert_eq!(err.line, 2);
        assert_eq!(
            err.kind,
            FormatErrorKind::UnknownKey {
                path: String::new(),
                key: "nmae".to_owned()
            }
        );
    }

    #[test]
    fn a_missing_required_key_is_reported() {
        let root = parse("a: 1\nname:\n").unwrap();
        let mut map = MapReader::new(&root, "").unwrap();
        assert_eq!(
            map.required_string("name").unwrap_err().kind,
            FormatErrorKind::MissingKey {
                path: String::new(),
                key: "name".to_owned()
            }
        );
    }
}

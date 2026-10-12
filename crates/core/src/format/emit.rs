//! The deterministic YAML writer (D-008).
//!
//! Files are built as a [`Value`] tree and rendered by one function, so the
//! same model always gives the same bytes. The output uses a small subset of
//! YAML: block mappings and sequences, plain scalars, double-quoted scalars
//! and `|` literal block scalars. No flow collections, anchors, aliases,
//! tags or comments. Two-space indentation, LF line endings, one trailing
//! newline.

/// A node of a file about to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Str(String),
    Bool(bool),
    Int(u64),
    /// Entries are written in the given order.
    Map(Vec<(String, Value)>),
    List(Vec<Value>),
}

impl Value {
    pub(crate) fn str(value: impl Into<String>) -> Self {
        Value::Str(value.into())
    }

    /// Empty collections are never written: a key whose value is empty is
    /// left out, the same as any other default.
    fn is_empty_collection(&self) -> bool {
        match self {
            Value::Map(entries) => entries.is_empty(),
            Value::List(items) => items.is_empty(),
            _ => false,
        }
    }
}

/// Builds the entries of a mapping in a fixed order.
#[derive(Debug, Default)]
pub(crate) struct MapBuilder(Vec<(String, Value)>);

impl MapBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Always writes the entry.
    pub(crate) fn put(&mut self, key: &str, value: Value) -> &mut Self {
        self.0.push((key.to_owned(), value));
        self
    }

    /// Writes a string unless it is empty.
    pub(crate) fn put_str(&mut self, key: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            self.put(key, Value::str(value));
        }
        self
    }

    /// Writes `key: false` only; `true` is the default.
    pub(crate) fn put_disabled(&mut self, key: &str, enabled: bool) -> &mut Self {
        if !enabled {
            self.put(key, Value::Bool(false));
        }
        self
    }

    pub(crate) fn build(self) -> Value {
        Value::Map(self.0)
    }
}

const INDENT: usize = 2;

/// Renders a top-level mapping as a complete file.
pub(crate) fn render(document: &Value) -> String {
    let mut out = String::new();
    match document {
        Value::Map(entries) => write_map(&mut out, entries, 0, false),
        // Every file is a mapping; this branch only keeps the function total.
        other => write_scalar_line(&mut out, other, 0),
    }
    out
}

/// Writes mapping entries at `indent`. When `inline_first` is set, the first
/// entry continues the current line (after a `- ` list marker).
fn write_map(out: &mut String, entries: &[(String, Value)], indent: usize, inline_first: bool) {
    let mut first = true;
    for (key, value) in entries {
        if value.is_empty_collection() {
            continue;
        }
        if !(first && inline_first) {
            push_indent(out, indent);
        }
        first = false;
        out.push_str(&scalar_inline(key));
        out.push(':');
        match value {
            Value::Map(children) => {
                out.push('\n');
                write_map(out, children, indent + INDENT, false);
            }
            Value::List(items) => {
                out.push('\n');
                write_list(out, items, indent + INDENT);
            }
            scalar => {
                out.push(' ');
                write_scalar_line(out, scalar, indent + INDENT);
            }
        }
    }
}

fn write_list(out: &mut String, items: &[Value], indent: usize) {
    for item in items {
        push_indent(out, indent);
        out.push('-');
        match item {
            Value::Map(entries) if entries.iter().any(|(_, v)| !v.is_empty_collection()) => {
                out.push(' ');
                write_map(out, entries, indent + INDENT, true);
            }
            Value::Map(_) | Value::List(_) => {
                // Writers never produce empty or nested list items; an
                // explicit null keeps the output valid if one ever does.
                out.push_str(" null\n");
            }
            scalar => {
                out.push(' ');
                write_scalar_line(out, scalar, indent + INDENT);
            }
        }
    }
}

/// Writes a scalar after `key: ` or `- `, including the line break. A
/// multi-line string becomes a literal block indented to `block_indent`.
fn write_scalar_line(out: &mut String, value: &Value, block_indent: usize) {
    match value {
        Value::Str(s) if s.contains('\n') && can_be_literal_block(s) => {
            write_literal_block(out, s, block_indent);
        }
        Value::Str(s) => {
            out.push_str(&scalar_inline(s));
            out.push('\n');
        }
        Value::Bool(b) => {
            out.push_str(if *b { "true" } else { "false" });
            out.push('\n');
        }
        Value::Int(n) => {
            out.push_str(&n.to_string());
            out.push('\n');
        }
        Value::Map(_) | Value::List(_) => out.push_str("null\n"),
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

/// A single-line scalar: plain when that reads back as the same string in
/// YAML 1.1 and 1.2, double-quoted otherwise.
pub(crate) fn scalar_inline(s: &str) -> String {
    if is_safe_plain(s) {
        s.to_owned()
    } else {
        double_quoted(s)
    }
}

/// Characters that YAML does not allow unescaped, or that are invisible and
/// would be lost or confusing in a file. These force a double-quoted scalar.
fn needs_escape(c: char) -> bool {
    (c.is_control() && c != '\t' && c != '\n')
        || matches!(
            c,
            '\u{2028}' | '\u{2029}' | '\u{FEFF}' | '\u{FFFE}' | '\u{FFFF}'
        )
}

/// Words that a YAML 1.1 or 1.2 reader resolves to null or a boolean.
/// Compared case-insensitively, which also covers forms like `True`.
const RESERVED_WORDS: &[&str] = &[
    "~", "null", "true", "false", "yes", "no", "on", "off", "y", "n", "<<", "=",
];

/// Conservative: when in doubt, quote. A string is written plain only if
/// no YAML reader can take it for anything but the same string.
fn is_safe_plain(s: &str) -> bool {
    let (Some(first), Some(last)) = (s.chars().next(), s.chars().last()) else {
        return false;
    };
    if s.chars().any(|c| needs_escape(c) || c == '\t' || c == '\n') {
        return false;
    }
    if " -?:,[]{}#&*!|>'\"%@`".contains(first) || last == ' ' || last == ':' {
        return false;
    }
    if s.contains(": ") || s.contains(" #") {
        return false;
    }
    if RESERVED_WORDS.contains(&s.to_ascii_lowercase().as_str()) {
        return false;
    }
    // Anything that might be read as a number, a date, a time or a special
    // float (`1e3`, `12:30`, `+1`, `.5`, `.inf`), or as a document marker
    // (`...`). Ids are the exception: no YAML reader takes a UUID for a number.
    !(first.is_ascii_digit() || matches!(first, '+' | '.')) || is_uuid(s)
}

/// `8-4-4-4-12` hex digits, the shape of every id we write.
fn is_uuid(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, len)| g.len() == len && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub(crate) fn double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c if needs_escape(c) => {
                let code = u32::from(c);
                if code <= 0xFF {
                    out.push_str(&format!("\\x{code:02X}"));
                } else {
                    out.push_str(&format!("\\u{code:04X}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A literal block keeps every character as written, so it is used only
/// for text it can represent: no characters that need escaping, no
/// carriage returns, and at least one character besides line breaks (a
/// block of only empty lines reads back as an empty string).
fn can_be_literal_block(s: &str) -> bool {
    !s.trim_end_matches('\n').is_empty()
        && !s.chars().any(|c| c != '\t' && c != '\n' && needs_escape(c))
}

/// Writes `|`, `|-` or `|+` (with an indentation indicator when the first
/// non-empty line starts with a space) and the lines of `s`.
fn write_literal_block(out: &mut String, s: &str, indent: usize) {
    let body = s.trim_end_matches('\n');
    let trailing_newlines = s.len() - body.len();

    out.push('|');
    let first_content_line = body.split('\n').find(|line| !line.is_empty());
    if first_content_line.is_some_and(|line| line.starts_with(' ')) {
        out.push_str(&INDENT.to_string());
    }
    match trailing_newlines {
        0 => out.push('-'),
        1 => {}
        _ => out.push('+'),
    }
    out.push('\n');

    for line in body.split('\n') {
        if !line.is_empty() {
            push_indent(out, indent);
            out.push_str(line);
        }
        out.push('\n');
    }
    for _ in 1..trailing_newlines {
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, Value)]) -> Value {
        Value::Map(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn plain_when_safe() {
        for s in [
            "Get pet by ID",
            "GET",
            "a0",
            "application/json",
            "https://example.com/a?b=c",
            "Bearer abc.def",
            "élan",
            "a:b",
            "3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70",
            "a#b",
            "x-api-key",
            "v1.2",
            "localhost:8080",
        ] {
            assert_eq!(scalar_inline(s), s, "{s:?}");
        }
    }

    #[test]
    fn quoted_when_a_reader_could_misread_it() {
        for s in [
            "",
            " a",
            "a ",
            "true",
            "False",
            "no",
            "ON",
            "y",
            "null",
            "~",
            "123",
            "1.5",
            "0x1F",
            "2024-01-01",
            "3f2b8c1e-7a4d-4e9b-9c55",
            "12:30",
            "+1",
            ".5",
            ".inf",
            "-1",
            "-a",
            "{{baseUrl}}/pets",
            "[a]",
            "#comment",
            "a: b",
            "a #b",
            "key:",
            "'quoted'",
            "\"quoted\"",
            "*alias",
            "&anchor",
            "!tag",
            "|",
            ">",
            "%",
            "@",
            "`",
            "a\tb",
            "...",
            "<<",
            "=",
        ] {
            assert!(scalar_inline(s).starts_with('"'), "{s:?}");
        }
    }

    #[test]
    fn escapes_in_double_quotes() {
        assert_eq!(double_quoted("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(double_quoted("\r\n\t\0"), r#""\r\n\t\0""#);
        assert_eq!(
            double_quoted("\u{7}\u{85}\u{2028}"),
            "\"\\x07\\x85\\u2028\""
        );
        assert_eq!(double_quoted("é"), "\"é\"");
    }

    #[test]
    fn layout_of_maps_and_lists() {
        let doc = map(&[
            ("version", Value::Int(1)),
            ("name", Value::str("Petstore")),
            ("empty", Value::List(Vec::new())),
            (
                "query",
                Value::List(vec![
                    map(&[("name", Value::str("a")), ("enabled", Value::Bool(false))]),
                    map(&[("name", Value::str("b"))]),
                ]),
            ),
            ("auth", map(&[("type", Value::str("bearer"))])),
        ]);
        assert_eq!(
            render(&doc),
            "version: 1\nname: Petstore\nquery:\n  - name: a\n    enabled: false\n  - name: b\nauth:\n  type: bearer\n"
        );
    }

    #[test]
    fn literal_blocks_choose_their_chomping() {
        let doc = |s: &str| render(&map(&[("content", Value::str(s))]));
        assert_eq!(doc("a\nb\n"), "content: |\n  a\n  b\n");
        assert_eq!(doc("a\nb"), "content: |-\n  a\n  b\n");
        assert_eq!(doc("a\n\n"), "content: |+\n  a\n\n");
        assert_eq!(doc("a\n\nb"), "content: |-\n  a\n\n  b\n");
        assert_eq!(doc("  a\nb"), "content: |2-\n    a\n  b\n");
        assert_eq!(doc("a\r\nb"), "content: \"a\\r\\nb\"\n");
    }
}

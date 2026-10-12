//! Parses a file into a small tree of mappings, sequences and scalars, with
//! line numbers for error messages.
//!
//! Parsing is done by `saphyr-parser` (D-008, D-034). This module accepts
//! any YAML 1.2 document that maps onto plain data, so hand edits in other
//! styles (flow collections, single quotes, folded blocks) still load. It
//! rejects what our files never need and what makes data ambiguous:
//! anchors, aliases, tags, several documents and duplicate keys.

use saphyr_parser::{Event, Parser, ScalarStyle, Span};

use super::error::{FormatError, FormatErrorKind};

/// Deeper nesting than any valid file needs. The limit keeps a hostile file
/// from exhausting the stack.
const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) kind: NodeKind,
    /// 1-based position of the node's first character.
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// `plain` is false for quoted and block scalars: only a plain scalar
    /// can mean null, a boolean or a number.
    Scalar {
        value: String,
        plain: bool,
    },
    Map(Vec<Entry>),
    List(Vec<Node>),
}

/// A mapping entry, with the key's own position for errors about the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) key: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) value: Node,
}

impl Entry {
    pub(crate) fn error(&self, kind: FormatErrorKind) -> FormatError {
        FormatError {
            line: self.line,
            column: self.column,
            kind,
        }
    }
}

impl Node {
    pub(crate) fn error(&self, kind: FormatErrorKind) -> FormatError {
        FormatError {
            line: self.line,
            column: self.column,
            kind,
        }
    }

    /// `true` for an empty plain scalar or `~` / `null`, which YAML reads as null.
    pub(crate) fn is_null(&self) -> bool {
        matches!(&self.kind, NodeKind::Scalar { value, plain: true }
            if matches!(value.as_str(), "" | "~" | "null" | "Null" | "NULL"))
    }
}

/// Parses `text` into the root node. The root of every file is a mapping.
pub(crate) fn parse(text: &str) -> Result<Node, FormatError> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut builder = Builder {
        events: Parser::new_from_str(text),
        last_line: 1,
    };
    builder.document()
}

struct Builder<'a> {
    events: Parser<'a, saphyr_parser::StrInput<'a>>,
    last_line: usize,
}

impl<'a> Builder<'a> {
    fn next(&mut self) -> Result<(Event<'a>, Span), FormatError> {
        match self.events.next_event() {
            Some(Ok((event, span))) => {
                self.last_line = span.start.line();
                Ok((event, span))
            }
            Some(Err(e)) => Err(FormatError {
                line: e.marker().line(),
                column: e.marker().col() + 1,
                kind: FormatErrorKind::Syntax(e.info().to_owned()),
            }),
            None => Err(self.error_here(FormatErrorKind::Syntax(
                "unexpected end of input".to_owned(),
            ))),
        }
    }

    fn error_here(&self, kind: FormatErrorKind) -> FormatError {
        FormatError {
            line: self.last_line,
            column: 1,
            kind,
        }
    }

    fn document(&mut self) -> Result<Node, FormatError> {
        let mut root = None;
        loop {
            let (event, span) = self.next()?;
            match event {
                Event::StreamStart | Event::DocumentEnd => {}
                Event::DocumentStart(_) if root.is_some() => {
                    return Err(at(span, FormatErrorKind::Unsupported("several documents")));
                }
                Event::DocumentStart(_) => {
                    let (event, span) = self.next()?;
                    root = Some(self.node(event, span, 0)?);
                }
                Event::StreamEnd => break,
                _ => {
                    return Err(at(
                        span,
                        FormatErrorKind::Syntax("unexpected content".to_owned()),
                    ));
                }
            }
        }
        match root {
            None => Err(FormatError {
                line: 1,
                column: 1,
                kind: FormatErrorKind::Empty,
            }),
            Some(node) if node.is_null() => Err(node.error(FormatErrorKind::Empty)),
            Some(node) if !matches!(node.kind, NodeKind::Map(_)) => {
                Err(node.error(FormatErrorKind::WrongType {
                    path: String::new(),
                    expected: "a mapping",
                }))
            }
            Some(node) => Ok(node),
        }
    }

    fn node(&mut self, event: Event<'a>, span: Span, depth: usize) -> Result<Node, FormatError> {
        if depth > MAX_DEPTH {
            return Err(at(span, FormatErrorKind::Unsupported("nesting this deep")));
        }
        let kind = match event {
            Event::Scalar(value, style, anchor, tag) => {
                check_plain_data(span, anchor, tag.is_some())?;
                NodeKind::Scalar {
                    value: value.into_owned(),
                    plain: style == ScalarStyle::Plain,
                }
            }
            Event::SequenceStart(anchor, tag) => {
                check_plain_data(span, anchor, tag.is_some())?;
                let mut items = Vec::new();
                loop {
                    let (event, span) = self.next()?;
                    if matches!(event, Event::SequenceEnd) {
                        break;
                    }
                    items.push(self.node(event, span, depth + 1)?);
                }
                NodeKind::List(items)
            }
            Event::MappingStart(anchor, tag) => {
                check_plain_data(span, anchor, tag.is_some())?;
                let mut entries: Vec<Entry> = Vec::new();
                loop {
                    let (event, key_span) = self.next()?;
                    let key = match event {
                        Event::MappingEnd => break,
                        Event::Scalar(key, _, anchor, tag) => {
                            check_plain_data(key_span, anchor, tag.is_some())?;
                            key.into_owned()
                        }
                        Event::Alias(_) => {
                            return Err(at(key_span, FormatErrorKind::Unsupported("aliases")));
                        }
                        _ => {
                            return Err(at(key_span, FormatErrorKind::Unsupported("complex keys")));
                        }
                    };
                    if entries.iter().any(|e| e.key == key) {
                        return Err(at(key_span, FormatErrorKind::DuplicateKey(key)));
                    }
                    let (event, span) = self.next()?;
                    let value = self.node(event, span, depth + 1)?;
                    entries.push(Entry {
                        key,
                        line: key_span.start.line(),
                        column: key_span.start.col() + 1,
                        value,
                    });
                }
                NodeKind::Map(entries)
            }
            Event::Alias(_) => return Err(at(span, FormatErrorKind::Unsupported("aliases"))),
            _ => {
                return Err(at(
                    span,
                    FormatErrorKind::Syntax("unexpected content".to_owned()),
                ));
            }
        };
        Ok(Node {
            kind,
            line: span.start.line(),
            column: span.start.col() + 1,
        })
    }
}

fn at(span: Span, kind: FormatErrorKind) -> FormatError {
    FormatError {
        line: span.start.line(),
        column: span.start.col() + 1,
        kind,
    }
}

/// Anchors and tags have no meaning in our files. An anchor id of 0 means
/// "no anchor".
fn check_plain_data(span: Span, anchor: usize, tagged: bool) -> Result<(), FormatError> {
    if anchor != 0 {
        return Err(at(span, FormatErrorKind::Unsupported("anchors")));
    }
    if tagged {
        return Err(at(span, FormatErrorKind::Unsupported("tags")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(node: &Node) -> (&str, bool) {
        match &node.kind {
            NodeKind::Scalar { value, plain } => (value, *plain),
            other => panic!("not a scalar: {other:?}"),
        }
    }

    fn entries(node: &Node) -> &[Entry] {
        match &node.kind {
            NodeKind::Map(entries) => entries,
            other => panic!("not a map: {other:?}"),
        }
    }

    fn kind_of(text: &str) -> FormatErrorKind {
        parse(text).unwrap_err().kind
    }

    #[test]
    fn builds_a_tree_with_positions() {
        let root = parse("a: 1\nb:\n  - x\n  - 'y'\n").unwrap();
        let e = entries(&root);
        assert_eq!(e[0].key, "a");
        assert_eq!(scalar(&e[0].value), ("1", true));
        assert_eq!((e[1].line, e[1].column), (2, 1));
        assert_eq!((e[1].value.line, e[1].value.column), (3, 3));
        let NodeKind::List(items) = &e[1].value.kind else {
            panic!()
        };
        assert_eq!(scalar(&items[1]), ("y", false));
    }

    #[test]
    fn hand_edits_in_other_styles_load() {
        let root =
            parse("\u{FEFF}a: [1, 2]\r\nb: {c: d}\r\ne: >\r\n  folded\r\n  text\r\n").unwrap();
        let e = entries(&root);
        assert!(matches!(&e[0].value.kind, NodeKind::List(items) if items.len() == 2));
        assert_eq!(scalar(&e[2].value), ("folded text\n", false));
    }

    #[test]
    fn rejects_what_files_never_need() {
        assert_eq!(kind_of(""), FormatErrorKind::Empty);
        assert_eq!(kind_of("# only a comment\n"), FormatErrorKind::Empty);
        assert_eq!(kind_of("~\n"), FormatErrorKind::Empty);
        assert_eq!(
            kind_of("a: &x 1\nb: *x\n"),
            FormatErrorKind::Unsupported("anchors")
        );
        assert_eq!(
            kind_of("a: !!str 1\n"),
            FormatErrorKind::Unsupported("tags")
        );
        assert_eq!(
            kind_of("a: 1\n---\nb: 2\n"),
            FormatErrorKind::Unsupported("several documents")
        );
        assert_eq!(
            kind_of("? [a]\n: 1\n"),
            FormatErrorKind::Unsupported("complex keys")
        );
        assert_eq!(
            kind_of("a: 1\na: 2\n"),
            FormatErrorKind::DuplicateKey("a".to_owned())
        );
        assert!(matches!(
            kind_of("- a\n"),
            FormatErrorKind::WrongType { .. }
        ));
        assert!(matches!(kind_of("a: [\n"), FormatErrorKind::Syntax(_)));
    }

    #[test]
    fn deep_nesting_is_refused_not_overflowed() {
        let text = format!("a: {}{}\n", "[".repeat(40), "]".repeat(40));
        assert_eq!(
            kind_of(&text),
            FormatErrorKind::Unsupported("nesting this deep")
        );
        // Much deeper input is stopped by the parser's own limit.
        let text = format!("a: {}{}\n", "[".repeat(100_000), "]".repeat(100_000));
        assert!(matches!(kind_of(&text), FormatErrorKind::Syntax(_)));
    }

    #[test]
    fn syntax_errors_carry_a_line() {
        let err = parse("a: 1\nb: \"unterminated\n").unwrap_err();
        assert!(matches!(err.kind, FormatErrorKind::Syntax(_)));
        assert!(err.line >= 2, "{err:?}");
    }
}

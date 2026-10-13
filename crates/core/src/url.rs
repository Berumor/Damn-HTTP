//! The one parser for URL templates (D-013, D-028).
//!
//! A URL template is what the user types in the URL bar: a URL that may
//! contain `{{variable}}` references and `:pathVariable` segments. The file
//! loader, the request model and (through the `url_analyze` command) the UI
//! all use this module, so they can never disagree about what a path
//! variable is.
//!
//! Rules:
//!
//! - Text inside `{{ }}` is opaque: nothing in it is a path variable, a
//!   query separator or a fragment.
//! - The authority is skipped: after `scheme://` or a leading `//` up to the
//!   next `/`; or, with no scheme, everything before the first `/` (so
//!   `localhost:8080/x` and `{{baseUrl}}/x` work).
//! - A path variable is a whole path segment of the form `:name`, as in
//!   Postman: the name runs to the next `/` or the end of the path, so
//!   `/files/:name.pdf` has the variable `name.pdf` (D-040). `name` matches
//!   `[A-Za-z_][A-Za-z0-9_.-]*`; a segment that starts with `:` but is not
//!   such a name (`/:1x`, `/:a:b`, `/:id{{x}}`) is ordinary text.
//! - The path ends at the first `?` or `#`.
//!
//! Positions are byte offsets into the template.

use std::ops::Range;

use crate::reference::is_valid_path_variable_name;

/// A `:name` path variable in a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathVariableSpan {
    pub name: String,
    /// Covers the colon and the name.
    pub range: Range<usize>,
}

/// A `{{name}}` reference in a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSpan {
    /// The text between the braces, trimmed.
    pub name: String,
    /// `false` when `name` is not a valid variable name (for example `{{a b}}`).
    pub valid: bool,
    /// Covers the braces.
    pub range: Range<usize>,
}

/// Where the parts of a URL template are.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UrlAnalysis {
    pub path_variables: Vec<PathVariableSpan>,
    pub references: Vec<ReferenceSpan>,
    /// The query string without the `?`, if the template has one.
    pub query: Option<Range<usize>>,
    /// The fragment including the `#`, if the template has one.
    pub fragment: Option<Range<usize>>,
}

impl UrlAnalysis {
    /// The path variable names in order of first appearance, without repeats.
    pub fn path_variable_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for span in &self.path_variables {
            if !names.contains(&span.name.as_str()) {
                names.push(&span.name);
            }
        }
        names
    }
}

/// Finds the references, path variables, query string and fragment of a template.
pub fn analyze(url: &str) -> UrlAnalysis {
    let bytes = url.as_bytes();
    let (opaque, references) = scan_references(url);
    // A byte counts as URL syntax only outside `{{ }}`.
    let is = |index: usize, byte: u8| bytes.get(index) == Some(&byte) && !opaque[index];
    let find = |from: usize, wanted: &[u8]| {
        (from..bytes.len()).find(|&i| !opaque[i] && wanted.contains(&bytes[i]))
    };

    let authority_start = match find(0, b"/?#") {
        // `scheme://`: the first delimiter is the first slash after a colon.
        Some(slash)
            if slash > 0 && is(slash - 1, b':') && is(slash, b'/') && is(slash + 1, b'/') =>
        {
            Some(slash + 2)
        }
        Some(0) if is(0, b'/') && is(1, b'/') => Some(2),
        // A leading `/`, `?` or `#`: no authority.
        Some(0) => None,
        // No scheme: the template starts with the host or a reference to it.
        _ => Some(0),
    };
    let path_start = match authority_start {
        Some(start) => find(start, b"/?#").unwrap_or(bytes.len()),
        None => 0,
    };
    let path_end = find(path_start, b"?#").unwrap_or(bytes.len());

    let fragment_start = find(path_end, b"#");
    let fragment = fragment_start.map(|start| start..bytes.len());
    let query = (is(path_end, b'?')).then(|| path_end + 1..fragment_start.unwrap_or(bytes.len()));

    let mut path_variables = Vec::new();
    let mut index = path_start;
    while index < path_end {
        if !is(index, b'/') {
            index += 1;
            continue;
        }
        // The segment runs to the next `/` or the end of the path.
        let segment_start = index + 1;
        let segment_end = find(segment_start, b"/")
            .filter(|end| *end < path_end)
            .unwrap_or(path_end);
        let name = segment_start + 1..segment_end;
        // A reference inside the segment makes it ordinary text: opaque
        // bytes are never part of a name.
        if is(segment_start, b':')
            && !opaque[name.clone()].contains(&true)
            && is_valid_path_variable_name(&url[name.clone()])
        {
            path_variables.push(PathVariableSpan {
                name: url[name].to_owned(),
                range: segment_start..segment_end,
            });
        }
        index = segment_end;
    }

    UrlAnalysis {
        path_variables,
        references,
        query,
        fragment,
    }
}

/// Marks every byte inside a `{{ ... }}` (braces included) and lists the
/// references. An unclosed `{{` is ordinary text.
fn scan_references(text: &str) -> (Vec<bool>, Vec<ReferenceSpan>) {
    let mut opaque = vec![false; text.len()];
    let mut references = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("{{").map(|i| from + i) {
        let Some(close) = text[open + 2..].find("}}").map(|i| open + 2 + i) else {
            break;
        };
        let end = close + 2;
        let name = text[open + 2..close].trim();
        references.push(ReferenceSpan {
            name: name.to_owned(),
            valid: crate::reference::is_valid_variable_name(name),
            range: open..end,
        });
        for flag in &mut opaque[open..end] {
            *flag = true;
        }
        from = end;
    }
    (opaque, references)
}

/// A URL template with its query string taken out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitUrl {
    /// The template without `?query`. A fragment is kept.
    pub url: String,
    /// The query parameters as `(name, value)`, exactly as written: nothing
    /// is percent-decoded. A parameter without `=` has an empty value.
    pub query: Vec<(String, String)>,
}

/// Separates the query string from a typed or pasted URL (D-021): the URL
/// goes to the request's `url`, the parameters to its query table.
pub fn split_query(input: &str) -> SplitUrl {
    let analysis = analyze(input);
    let Some(query_range) = analysis.query else {
        return SplitUrl {
            url: input.to_owned(),
            query: Vec::new(),
        };
    };
    let (opaque, _) = scan_references(input);
    let bytes = input.as_bytes();
    let mut query = Vec::new();
    let mut pair_start = query_range.start;
    for index in query_range.start..=query_range.end {
        let at_separator = index == query_range.end || (bytes[index] == b'&' && !opaque[index]);
        if !at_separator {
            continue;
        }
        let pair = pair_start..index;
        pair_start = index + 1;
        if pair.is_empty() {
            continue;
        }
        let equals = pair.clone().find(|&i| bytes[i] == b'=' && !opaque[i]);
        let (name, value) = match equals {
            Some(equals) => (&input[pair.start..equals], &input[equals + 1..pair.end]),
            None => (&input[pair], ""),
        };
        query.push((name.to_owned(), value.to_owned()));
    }
    let mut url = input[..query_range.start - 1].to_owned();
    url.push_str(&input[query_range.end..]);
    SplitUrl { url, query }
}

/// The reverse of [`split_query`]: the text for the URL bar. Parameters are
/// written as they are, `name=value` or just `name` when the value is empty.
pub fn join_query<'a>(url: &str, query: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut pairs = String::new();
    for (name, value) in query {
        pairs.push(if pairs.is_empty() { '?' } else { '&' });
        pairs.push_str(name);
        if !value.is_empty() {
            pairs.push('=');
            pairs.push_str(value);
        }
    }
    if pairs.is_empty() {
        return url.to_owned();
    }
    let analysis = analyze(url);
    // An existing query in `url` would make the result ambiguous; callers
    // keep `url` free of one, and we still produce a usable URL if not.
    let insert_at = match (&analysis.query, &analysis.fragment) {
        (Some(query), _) => {
            pairs.replace_range(0..1, if query.is_empty() { "" } else { "&" });
            query.end
        }
        (None, Some(fragment)) => fragment.start,
        (None, None) => url.len(),
    };
    let mut joined = url[..insert_at].to_owned();
    joined.push_str(&pairs);
    joined.push_str(&url[insert_at..]);
    joined
}

/// A template with its path variables replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substituted {
    pub url: String,
    /// Path variables that had no value and were left as `:name`, in order
    /// of first appearance.
    pub missing: Vec<String>,
}

/// Replaces each `:name` with its value from `value_of`.
///
/// Values are data, not URL syntax, so they are percent-encoded: everything
/// except `A-Z a-z 0-9 - . _ ~` is encoded, including `/` and `%`. A
/// `{{reference}}` inside a value is kept as it is, for the variable
/// resolver to fill in. A variable with no value, or an empty one, is left
/// in place and reported in [`Substituted::missing`].
pub fn substitute_path_variables<'a>(
    url: &str,
    value_of: impl Fn(&str) -> Option<&'a str>,
) -> Substituted {
    let analysis = analyze(url);
    let mut result = String::with_capacity(url.len());
    let mut missing: Vec<String> = Vec::new();
    let mut copied = 0;
    for span in &analysis.path_variables {
        match value_of(&span.name).filter(|value| !value.is_empty()) {
            Some(value) => {
                result.push_str(&url[copied..span.range.start]);
                encode_path_value(value, &mut result);
                copied = span.range.end;
            }
            None => {
                if !missing.contains(&span.name) {
                    missing.push(span.name.clone());
                }
            }
        }
    }
    result.push_str(&url[copied..]);
    Substituted {
        url: result,
        missing,
    }
}

fn encode_path_value(value: &str, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let (opaque, _) = scan_references(value);
    let mut encoded: Vec<u8> = Vec::with_capacity(value.len());
    for (index, byte) in value.bytes().enumerate() {
        if opaque[index] || byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(byte);
        } else {
            encoded.push(b'%');
            encoded.push(HEX[usize::from(byte >> 4)]);
            encoded.push(HEX[usize::from(byte & 0x0F)]);
        }
    }
    // References are copied whole and everything else is ASCII, so the
    // bytes are valid UTF-8; `lossy` only avoids a panic path.
    out.push_str(&String::from_utf8_lossy(&encoded));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(url: &str) -> Vec<String> {
        analyze(url)
            .path_variable_names()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn path_variables_are_found_after_a_slash_in_the_path() {
        assert_eq!(names("https://api.example.com/users/:id"), ["id"]);
        assert_eq!(
            names("https://some.service/:country/stations/"),
            ["country"]
        );
        assert_eq!(names("/a/:b/c/:_d1"), ["b", "_d1"]);
        assert_eq!(names("{{baseUrl}}/pets/:petId"), ["petId"]);
        assert_eq!(
            names("{{baseUrl}}/{{apiVersion}}/pets/:petId/toys/:toyId"),
            ["petId", "toyId"]
        );
        assert_eq!(names("localhost:8080/users/:id"), ["id"]);
        assert_eq!(names("example.com/:a"), ["a"]);
        assert_eq!(names("//example.com/:a"), ["a"]);
        assert_eq!(names("/:id/again/:id"), ["id"]);
    }

    #[test]
    fn the_name_is_the_whole_segment_as_in_postman() {
        assert_eq!(names("/files/:name.pdf"), ["name.pdf"]);
        assert_eq!(
            names("/users/:user-id/posts/:post_id.json"),
            ["user-id", "post_id.json"]
        );
        assert_eq!(names("/x/:a.b/"), ["a.b"]);
        assert_eq!(names("/x/:id?y=1"), ["id"]);
        assert_eq!(names("/x/:id#top"), ["id"]);
        let analysis = analyze("/files/:name.pdf/raw");
        assert_eq!(analysis.path_variables[0].range, 7..16);
    }

    #[test]
    fn a_segment_that_is_not_one_name_is_ordinary_text() {
        for url in [
            "/x/:a-:b",
            "/x/:a{{suffix}}",
            "/x/:{{name}}",
            "/x/:a b",
            "/x/:a@b",
            "/x/:a%20b",
            "/x/:.hidden",
            "/x/:-a",
            "/x/:idé",
        ] {
            assert_eq!(names(url), Vec::<String>::new(), "{url:?}");
        }
    }

    #[test]
    fn things_that_are_not_path_variables() {
        for url in [
            "localhost:8080",
            "localhost:8080/",
            "http://localhost:8080/users",
            "http://",
            "https://user:secret@example.com/x",
            "http://:secret@example.com/x",
            "//:8080/x",
            "/a:b",
            "/a/b:c",
            "/time/12:30",
            "/x/:1abc",
            "/x/:",
            "/x/::id",
            "/x?t=/:x",
            "/x?next=/users/:id",
            "/x#/:frag",
            "example.com?a=/:b",
            "{{a:b}}",
            "{{base}}/{{ /:notavar }}",
            "/x/{{a}}:b",
            "/x/%3Aid",
            "",
            ":id",
            "mailto:someone",
        ] {
            assert_eq!(names(url), Vec::<String>::new(), "{url:?}");
        }
    }

    #[test]
    fn references_are_listed_with_their_validity() {
        let analysis = analyze("{{ base }}/x/{{a b}}/{{unclosed");
        let found: Vec<(&str, bool, Range<usize>)> = analysis
            .references
            .iter()
            .map(|r| (r.name.as_str(), r.valid, r.range.clone()))
            .collect();
        assert_eq!(found, [("base", true, 0..10), ("a b", false, 13..20)]);
    }

    #[test]
    fn query_and_fragment_are_located() {
        let analysis = analyze("https://x.test/a?b=1&c=2#top");
        assert_eq!(analysis.query, Some(17..24));
        assert_eq!(analysis.fragment, Some(24..28));
        assert_eq!(analyze("https://x.test/a#top?not=query").query, None);
        assert_eq!(analyze("x.test?b=1").query, Some(7..10));
        assert_eq!(analyze("/a?").query, Some(3..3));
        assert_eq!(analyze("{{u}}/a").query, None);
        assert_eq!(
            analyze("/a/{{x?y}}").query,
            None,
            "a question mark inside a reference"
        );
    }

    /// The maintainer's example.
    #[test]
    fn a_pasted_url_is_split_into_url_and_parameters() {
        let split = split_query("https://some.service/:country/stations/?id=1234");
        assert_eq!(split.url, "https://some.service/:country/stations/");
        assert_eq!(split.query, [("id".to_owned(), "1234".to_owned())]);
    }

    #[test]
    fn query_splitting_details() {
        let pairs = |input: &str| -> Vec<(String, String)> { split_query(input).query };
        let pair = |n: &str, v: &str| (n.to_owned(), v.to_owned());

        assert_eq!(pairs("/a"), []);
        assert_eq!(pairs("/a?"), []);
        assert_eq!(pairs("/a?x=1&y=2"), [pair("x", "1"), pair("y", "2")]);
        assert_eq!(
            pairs("/a?flag&x=&&y=a=b"),
            [pair("flag", ""), pair("x", ""), pair("y", "a=b")]
        );
        assert_eq!(pairs("/a?id=1&id=2"), [pair("id", "1"), pair("id", "2")]);
        // Nothing is decoded: what was typed is what is kept.
        assert_eq!(
            pairs("/a?q=hello%20world+x"),
            [pair("q", "hello%20world+x")]
        );
        // Separators inside a reference belong to the reference.
        assert_eq!(
            pairs("/a?t={{a&b=c}}&u={{v}}"),
            [pair("t", "{{a&b=c}}"), pair("u", "{{v}}")]
        );

        let split = split_query("https://x.test/a?b=1#top");
        assert_eq!(split.url, "https://x.test/a#top");
        assert_eq!(split.query, [pair("b", "1")]);
        assert_eq!(
            split_query("https://x.test/a#top?b=1").url,
            "https://x.test/a#top?b=1"
        );
    }

    #[test]
    fn joining_is_the_reverse_of_splitting() {
        for input in [
            "https://some.service/:country/stations/?id=1234",
            "/a?x=1&y=2",
            "/a?flag&y=a=b",
            "https://x.test/a?b=1#top",
            "{{baseUrl}}/pets?tag={{tag}}&limit=20",
            "/no-query",
        ] {
            let split = split_query(input);
            let joined = join_query(
                &split.url,
                split.query.iter().map(|(n, v)| (n.as_str(), v.as_str())),
            );
            assert_eq!(joined, input);
        }
        assert_eq!(join_query("/a", [("x", "")]), "/a?x");
        assert_eq!(join_query("/a?b=1", [("c", "2")]), "/a?b=1&c=2");
        assert_eq!(join_query("/a?", [("c", "2")]), "/a?c=2");
    }

    #[test]
    fn substitution_encodes_values() {
        let value_of = |name: &str| match name {
            "country" => Some("it"),
            "id" => Some("a/b c%?é"),
            "ref" => Some("{{stationId}}"),
            "mixed" => Some("x {{a}}/y"),
            "empty" => Some(""),
            _ => None,
        };
        let sub = |url: &str| substitute_path_variables(url, value_of);

        assert_eq!(
            sub("https://s.test/:country/stations/").url,
            "https://s.test/it/stations/"
        );
        assert_eq!(sub("/x/:id").url, "/x/a%2Fb%20c%25%3F%C3%A9");
        assert_eq!(sub("/x/:ref/:mixed").url, "/x/{{stationId}}/x%20{{a}}%2Fy");
        assert_eq!(sub("/:country/x/:country").url, "/it/x/it");
        // The whole segment is one name, so this is a different variable.
        assert_eq!(sub("/:country.json").missing, ["country.json"]);

        let result = sub("/:country/:nope/:empty/:nope");
        assert_eq!(result.url, "/it/:nope/:empty/:nope");
        assert_eq!(result.missing, ["nope", "empty"]);

        // Ports and other colons are untouched.
        assert_eq!(
            sub("http://localhost:8080/:country").url,
            "http://localhost:8080/it"
        );
    }

    #[test]
    fn non_ascii_text_never_breaks_the_parser() {
        for url in [
            "https://exämple.test/pfad/:id?ü=ö#ä",
            "/日本語/:id/{{変数}}",
            "{{é}}/:a?b={{ü}}",
            "/:idé",
        ] {
            let analysis = analyze(url);
            for span in &analysis.path_variables {
                assert!(
                    url.is_char_boundary(span.range.start) && url.is_char_boundary(span.range.end)
                );
            }
            let split = split_query(url);
            assert!(!split.url.is_empty());
            let _ = substitute_path_variables(url, |_| Some("ü/ö"));
        }
        assert_eq!(names("/:idé"), Vec::<String>::new());
    }
}

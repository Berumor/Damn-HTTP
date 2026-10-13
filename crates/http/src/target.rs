//! Turning the URL text of a request into a URL that can be sent.

use damnhttp_core::url::analyze;
use url::Url;

use crate::error::{HttpError, HttpErrorKind};

/// Parses the URL of a request.
///
/// - `http://` is assumed when there is no scheme, so `localhost:8080/x`
///   and `example.com` work as typed.
/// - Only `http` and `https` are accepted.
/// - A `{{variable}}` that is still in the text is an error: sending it
///   percent-encoded would only produce a confusing response.
pub(crate) fn parse_target(text: &str) -> Result<Url, HttpError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(HttpError::new(
            HttpErrorKind::InvalidUrl,
            "the URL is empty",
        ));
    }
    let references = analyze(text).references;
    if !references.is_empty() {
        let names: Vec<&str> = references.iter().map(|r| r.name.as_str()).collect();
        return Err(HttpError::new(
            HttpErrorKind::UnresolvedVariables,
            names.join(", "),
        ));
    }

    let with_scheme = match scheme_of(text) {
        Some(scheme)
            if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") =>
        {
            text.to_owned()
        }
        Some(scheme) => {
            return Err(HttpError::new(HttpErrorKind::UnsupportedScheme, scheme));
        }
        None => format!("http://{text}"),
    };
    let url = Url::parse(&with_scheme)
        // The parser's message names the problem, not the URL.
        .map_err(|error| HttpError::new(HttpErrorKind::InvalidUrl, error.to_string()))?;
    if url.host_str().is_none_or(str::is_empty) {
        return Err(HttpError::new(
            HttpErrorKind::InvalidUrl,
            "the URL has no host",
        ));
    }
    Ok(url)
}

/// The scheme, if the text starts with `scheme://`.
fn scheme_of(text: &str) -> Option<&str> {
    let (scheme, _) = text.split_once("://")?;
    let mut chars = scheme.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some(scheme)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> String {
        parse_target(text).unwrap().to_string()
    }

    fn kind(text: &str) -> HttpErrorKind {
        parse_target(text).unwrap_err().kind
    }

    #[test]
    fn http_is_assumed_without_a_scheme() {
        assert_eq!(
            parsed("localhost:8080/users"),
            "http://localhost:8080/users"
        );
        assert_eq!(parsed("example.com"), "http://example.com/");
        assert_eq!(
            parsed("  127.0.0.1:3000/a?b=1 "),
            "http://127.0.0.1:3000/a?b=1"
        );
        assert_eq!(parsed("HTTPS://Example.com/x"), "https://example.com/x");
    }

    #[test]
    fn the_query_is_kept_as_typed() {
        assert_eq!(
            parsed("http://x.test/a?q=hello%20world+x&plus=%2B&a=b c&é=ü"),
            "http://x.test/a?q=hello%20world+x&plus=%2B&a=b%20c&%C3%A9=%C3%BC"
        );
        assert_eq!(
            parsed("http://x.test/a%2Fb/c d"),
            "http://x.test/a%2Fb/c%20d"
        );
    }

    #[test]
    fn other_schemes_and_broken_urls_are_refused() {
        assert_eq!(kind("ftp://x.test/file"), HttpErrorKind::UnsupportedScheme);
        assert_eq!(kind("file:///etc/passwd"), HttpErrorKind::UnsupportedScheme);
        assert_eq!(kind(""), HttpErrorKind::InvalidUrl);
        assert_eq!(kind("   "), HttpErrorKind::InvalidUrl);
        assert_eq!(kind("http://"), HttpErrorKind::InvalidUrl);
        assert_eq!(kind("http://exa mple.com/"), HttpErrorKind::InvalidUrl);
    }

    #[test]
    fn unresolved_variables_are_named() {
        let error = parse_target("{{baseUrl}}/pets?t={{ token }}").unwrap_err();
        assert_eq!(error.kind, HttpErrorKind::UnresolvedVariables);
        assert_eq!(error.detail, "baseUrl, token");
    }
}

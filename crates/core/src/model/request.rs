//! A single HTTP request as the user edits it.

use std::collections::HashSet;
use std::fmt;

use super::auth::Auth;
use super::id::RequestId;
use super::order::OrderKey;
use super::validation::ValidationError;
use crate::reference::is_valid_path_variable_name;

/// An HTTP method.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    /// Any other method token, stored as written (e.g. `PROPFIND`).
    Custom(String),
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Custom(method) => method,
        }
    }

    /// Maps the standard methods (written in upper case) to their variants;
    /// everything else becomes `Custom` and is checked by `Request::validate`.
    pub fn from_token(token: &str) -> Self {
        match token {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "PATCH" => Method::Patch,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            other => Method::Custom(other.to_owned()),
        }
    }

    /// A custom method must be a non-empty HTTP token (RFC 9110).
    fn is_valid(&self) -> bool {
        match self {
            Method::Custom(method) => {
                !method.is_empty()
                    && method
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            }
            _ => true,
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A query parameter row.
///
/// `value` holds whatever the user typed. Only a value that is exactly one
/// `{{name}}` is ever written to a committed file; a literal is stored
/// locally (D-021). See [`Request::split`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryParam {
    pub name: String,
    pub value: String,
    pub enabled: bool,
    pub description: String,
}

/// A value for a `:name` path variable in the URL. Same value rule as
/// [`QueryParam`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathParam {
    pub name: String,
    pub value: String,
    pub description: String,
}

/// A request header row. Header values are committed as written (D-025).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
    pub enabled: bool,
    pub description: String,
}

/// The request body. Bodies are committed as written (D-025).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Body {
    #[default]
    None,
    Json(String),
    Text {
        content: String,
        /// `None` means `text/plain`.
        content_type: Option<String>,
    },
    FormUrlencoded(Vec<Part>),
    Multipart(Vec<Part>),
}

/// One field of a form body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub value: PartValue,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartValue {
    Text(String),
    /// A file path relative to the workspace root. Multipart only.
    File(String),
}

/// Per-request transport settings that are shared with the team. `None`
/// means "use the app default". Proxy, custom CA and TLS verification are
/// local settings and are not here (D-020).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestSettings {
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<u32>,
    pub timeout_ms: Option<u64>,
}

/// A request with everything the editor shows, including values that are
/// only stored locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: RequestId,
    pub name: String,
    pub order: OrderKey,
    pub description: String,
    pub method: Method,
    /// The URL template, without a query string. May contain `{{var}}` and
    /// `:pathVariable`.
    pub url: String,
    pub path_params: Vec<PathParam>,
    pub query: Vec<QueryParam>,
    pub headers: Vec<Header>,
    pub auth: Auth,
    pub body: Body,
    pub settings: RequestSettings,
    /// Local only (D-020): never written to the request file.
    pub skip_tls_verify: bool,
}

impl Request {
    /// A new, empty `GET` request with a freshly generated id.
    pub fn new(name: impl Into<String>, order: OrderKey) -> Self {
        Self {
            id: RequestId::generate(),
            name: name.into(),
            order,
            description: String::new(),
            method: Method::default(),
            url: String::new(),
            path_params: Vec::new(),
            query: Vec::new(),
            headers: Vec::new(),
            auth: Auth::default(),
            body: Body::default(),
            settings: RequestSettings::default(),
            skip_tls_verify: false,
        }
    }

    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push(ValidationError::EmptyName);
        }
        if !self.method.is_valid() {
            errors.push(ValidationError::InvalidMethod(
                self.method.as_str().to_owned(),
            ));
        }
        let mut seen = HashSet::new();
        for param in &self.path_params {
            if !is_valid_path_variable_name(&param.name) {
                errors.push(ValidationError::InvalidPathVariableName(param.name.clone()));
            } else if !seen.insert(param.name.as_str()) {
                errors.push(ValidationError::DuplicatePathVariable(param.name.clone()));
            }
        }
        if let Body::FormUrlencoded(parts) = &self.body {
            for part in parts {
                if matches!(part.value, PartValue::File(_)) {
                    errors.push(ValidationError::FileInUrlencodedBody(part.name.clone()));
                }
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Request {
        Request::new("Get pet", OrderKey::new("a0").unwrap())
    }

    #[test]
    fn a_new_request_is_valid() {
        assert_eq!(request().validate(), []);
    }

    #[test]
    fn methods_round_trip_through_their_token() {
        for token in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
            let method = Method::from_token(token);
            assert!(!matches!(method, Method::Custom(_)), "{token}");
            assert_eq!(method.as_str(), token);
        }
        assert_eq!(
            Method::from_token("PROPFIND"),
            Method::Custom("PROPFIND".to_owned())
        );
        // Methods are case-sensitive: "get" is not GET.
        assert_eq!(Method::from_token("get"), Method::Custom("get".to_owned()));
    }

    #[test]
    fn problems_are_all_reported() {
        let mut request = request();
        request.name = String::new();
        request.method = Method::Custom("BAD METHOD".to_owned());
        for name in ["id", "id", "1x"] {
            request.path_params.push(PathParam {
                name: name.to_owned(),
                value: String::new(),
                description: String::new(),
            });
        }
        assert_eq!(
            request.validate(),
            [
                ValidationError::EmptyName,
                ValidationError::InvalidMethod("BAD METHOD".to_owned()),
                ValidationError::DuplicatePathVariable("id".to_owned()),
                ValidationError::InvalidPathVariableName("1x".to_owned()),
            ]
        );
    }

    #[test]
    fn a_urlencoded_body_cannot_carry_a_file() {
        let file = Part {
            name: "avatar".to_owned(),
            value: PartValue::File("files/a.png".to_owned()),
            enabled: true,
        };
        let mut request = request();
        request.body = Body::Multipart(vec![file.clone()]);
        assert_eq!(request.validate(), []);
        request.body = Body::FormUrlencoded(vec![file]);
        assert_eq!(
            request.validate(),
            [ValidationError::FileInUrlencodedBody("avatar".to_owned())]
        );
    }

    #[test]
    fn an_empty_custom_method_is_invalid() {
        let mut request = request();
        request.method = Method::Custom(String::new());
        assert_eq!(
            request.validate(),
            [ValidationError::InvalidMethod(String::new())]
        );
    }
}

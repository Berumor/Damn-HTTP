//! Authentication for the requests being tested (not for git).

/// How a request authenticates.
///
/// Collections, folders and requests each carry one. `Inherit` means "use
/// what the parent uses"; resolving the chain is done when a request is sent.
/// New schemes (OAuth2, API key) are added as new variants.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Auth {
    /// Use the parent's auth. The default for folders and requests.
    #[default]
    Inherit,
    /// Send no credentials, even if a parent defines some.
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
}

impl Auth {
    pub fn is_inherit(&self) -> bool {
        matches!(self, Auth::Inherit)
    }
}

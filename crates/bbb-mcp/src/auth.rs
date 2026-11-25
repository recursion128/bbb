//! Who is calling, and what they are allowed to ask for.
//!
//! Vanilla owner: `AuthenticationHandler` (security/AuthenticationHandler.java),
//! which checks one bearer token against a configured secret and then grants
//! the connection everything. bbb splits authority into the scope ladder the
//! world enforces, because a management endpoint an agent can reach should be
//! able to hand out read access without handing out `command.execute`.
//!
//! Two more differences follow from the transport rather than from policy: MCP
//! runs over HTTP, so an `Origin` check is needed against DNS rebinding, and a
//! browser that is refused has to be told where the protected resource's
//! metadata lives.

use std::{net::SocketAddr, sync::Arc};

use ring::digest::{digest, SHA256};

use crate::{
    host::{ScopeLadder, ScopeOf},
    http::HeaderMap,
    oauth::OAuthResource,
};

/// One issued credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenGrant<L: ScopeLadder> {
    /// The principal every operation this token authorizes is audited under.
    pub principal: String,
    pub scope: ScopeOf<L>,
    /// The bearer secret. Never logged and never echoed.
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConfig<L: ScopeLadder> {
    /// The scope a credential-less request from a loopback peer is granted.
    /// `None` requires a token even locally.
    pub loopback_scope: Option<ScopeOf<L>>,
    pub tokens: Vec<TokenGrant<L>>,
    /// Origins a browser may use. Empty means no browser origin is accepted at
    /// all, which is the default: a request that carries no `Origin` is not
    /// browser-initiated and is unaffected.
    pub allowed_origins: Vec<String>,
    /// This endpoint as an OAuth 2.1 protected resource. Required before a
    /// remote address may be bound; absent means only the issued tokens above
    /// and the loopback grant are accepted.
    pub oauth: Option<OAuthResource>,
}

impl<L: ScopeLadder> Default for AuthConfig<L> {
    fn default() -> Self {
        Self {
            loopback_scope: Some(L::MANAGE),
            tokens: Vec::new(),
            allowed_origins: Vec::new(),
            oauth: None,
        }
    }
}

/// The authenticated caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal<L: ScopeLadder> {
    pub name: String,
    pub scope: ScopeOf<L>,
}

/// The principal a loopback request with no credential is audited under. It is
/// a real name rather than an empty one so an audit line never reads as if it
/// had no author.
pub const LOOPBACK_PRINCIPAL: &str = "loopback";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthFailure {
    /// No credential, and none could be inferred from the peer.
    MissingCredential,
    /// A credential that does not name a grant. Deliberately not distinguished
    /// from an expired or revoked one in the answer.
    InvalidCredential,
    /// A credential that is valid but not for this endpoint.
    Forbidden(String),
}

impl AuthFailure {
    pub fn status(&self) -> u16 {
        match self {
            Self::MissingCredential | Self::InvalidCredential => 401,
            Self::Forbidden(_) => 403,
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::MissingCredential => "this endpoint requires a bearer token",
            Self::InvalidCredential => "the bearer token was not accepted",
            Self::Forbidden(detail) => detail,
        }
    }
}

pub fn authenticate<L: ScopeLadder>(
    config: &AuthConfig<L>,
    headers: &HeaderMap,
    peer: SocketAddr,
    now_millis: i64,
) -> Result<Principal<L>, AuthFailure> {
    match parse_authorization(headers) {
        AuthorizationHeader::Bearer(credential) => {
            if let Some(principal) = match_static_token(&config.tokens, credential) {
                return Ok(principal);
            }
            // An issued secret is not an OAuth token and an OAuth token is not an
            // issued secret; a credential that is neither is refused once.
            match &config.oauth {
                Some(resource) => resource.validate(credential, now_millis),
                None => Err(AuthFailure::InvalidCredential),
            }
        }
        AuthorizationHeader::Unsupported => Err(AuthFailure::InvalidCredential),
        AuthorizationHeader::Absent => match config.loopback_scope {
            Some(scope) if peer.ip().is_loopback() => Ok(Principal {
                name: LOOPBACK_PRINCIPAL.to_string(),
                scope,
            }),
            _ => Err(AuthFailure::MissingCredential),
        },
    }
}

/// Async variant of [`authenticate`] that offloads OAuth RSA signature
/// verification onto Tokio's blocking pool so async workers stay responsive.
pub(crate) async fn authenticate_async_with_oauth<L: ScopeLadder>(
    config: &AuthConfig<L>,
    shared_oauth: Option<&Arc<OAuthResource>>,
    headers: &HeaderMap,
    peer: SocketAddr,
    now_millis: i64,
) -> Result<Principal<L>, AuthFailure> {
    match parse_authorization(headers) {
        AuthorizationHeader::Bearer(credential) => {
            if let Some(principal) = match_static_token(&config.tokens, credential) {
                return Ok(principal);
            }
            let resource = match (shared_oauth, &config.oauth) {
                (Some(resource), _) => Some(Arc::clone(resource)),
                (None, Some(resource)) => Some(Arc::new(resource.clone())),
                (None, None) => None,
            };
            match resource {
                Some(resource) => {
                    let credential = credential.to_string();
                    tokio::task::spawn_blocking(move || resource.validate(&credential, now_millis))
                        .await
                        .unwrap_or(Err(AuthFailure::InvalidCredential))
                }
                None => Err(AuthFailure::InvalidCredential),
            }
        }
        AuthorizationHeader::Unsupported => Err(AuthFailure::InvalidCredential),
        AuthorizationHeader::Absent => match config.loopback_scope {
            Some(scope) if peer.ip().is_loopback() => Ok(Principal {
                name: LOOPBACK_PRINCIPAL.to_string(),
                scope,
            }),
            _ => Err(AuthFailure::MissingCredential),
        },
    }
}

fn match_static_token<L: ScopeLadder>(
    tokens: &[TokenGrant<L>],
    credential: &str,
) -> Option<Principal<L>> {
    // Every grant is compared, and each comparison is constant time, so a
    // rejected token leaks neither which grant it nearly matched nor its length.
    let mut matched = None;
    for grant in tokens {
        if secret_matches(&grant.secret, credential) {
            matched = Some(grant);
        }
    }
    matched.map(|grant| Principal {
        name: grant.principal.clone(),
        scope: grant.scope,
    })
}

enum AuthorizationHeader<'a> {
    Absent,
    Bearer(&'a str),
    Unsupported,
}

fn parse_authorization(headers: &HeaderMap) -> AuthorizationHeader<'_> {
    let Some(value) = headers.get("authorization") else {
        return AuthorizationHeader::Absent;
    };
    match value.split_once(' ') {
        Some((scheme, credential)) if scheme.eq_ignore_ascii_case("bearer") => {
            AuthorizationHeader::Bearer(credential.trim())
        }
        _ => AuthorizationHeader::Unsupported,
    }
}

/// Constant-time comparison via SHA-256 digests so neither secret contents nor
/// secret length leaks through an early exit.
pub(crate) fn secret_matches(secret: &str, candidate: &str) -> bool {
    let left_digest = digest(&SHA256, secret.as_bytes());
    let right_digest = digest(&SHA256, candidate.as_bytes());
    let mut difference = u8::from(secret.len() != candidate.len());
    for (left, right) in left_digest.as_ref().iter().zip(right_digest.as_ref()) {
        difference |= left ^ right;
    }
    std::hint::black_box(difference) == 0
}

/// The `WWW-Authenticate` challenge a refusal carries.
///
/// A protected resource points at its own metadata (RFC 9728) so a client that
/// has no token yet learns which authorization server to ask, and names the
/// OAuth error so a client with a stale token knows to refresh rather than to
/// give up.
pub fn challenge<L: ScopeLadder>(config: &AuthConfig<L>, failure: &AuthFailure) -> String {
    let mut challenge = String::from("Bearer");
    let error = match failure {
        AuthFailure::MissingCredential => None,
        AuthFailure::InvalidCredential => Some("invalid_token"),
        AuthFailure::Forbidden(_) => Some("insufficient_scope"),
    };
    if let Some(error) = error {
        challenge.push_str(&format!(" error=\"{error}\""));
    }
    if let Some(resource) = &config.oauth {
        let separator = if error.is_some() { "," } else { "" };
        challenge.push_str(&format!(
            "{separator} resource_metadata=\"{}\"",
            resource.metadata_url()
        ));
    }
    challenge
}

/// Whether a browser-initiated request may be served.
///
/// An absent `Origin` is not a browser request and passes. A present one has to
/// be declared, which is what stops a page on another site from driving a
/// loopback management endpoint through the user's browser.
pub fn origin_allowed<L: ScopeLadder>(config: &AuthConfig<L>, headers: &HeaderMap) -> bool {
    match headers.get("origin") {
        None => true,
        Some(origin) => config
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin),
    }
}

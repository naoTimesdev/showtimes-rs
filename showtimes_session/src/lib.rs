#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use std::sync::LazyLock;
#[allow(clippy::disallowed_types)]
use std::{collections::HashSet, sync::Arc};

use jiff::ToSpan;
use jwt_lc_rs::errors::ValidationError;
use jwt_lc_rs::validator::Validator;
use serde::{Deserialize, Serialize};

pub mod manager;
pub mod oauth2;
pub mod signer;

/// Re-export the error type from the jsonwebtoken crate
pub use jwt_lc_rs::errors::Error as SessionError;
/// Re-export the error type from the jsonwebtoken crate
pub use jwt_lc_rs::errors::ValidationError as SessionValidationError;

/// The global type for [`jwt_lc_rs::Signer`] with [`Arc`]
pub type SharedSigner = Arc<jwt_lc_rs::Signer>;

// The issuer of the token, we use a LazyLock to ensure it's only created once
#[allow(clippy::disallowed_types)]
static VALID_API_AUDIENCES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    #[allow(clippy::disallowed_types)]
    let mut set = HashSet::new();
    set.insert(ShowtimesAudience::APIKey.to_string());
    set.insert(ShowtimesAudience::MasterKey.to_string());
    set
});
const REFRESH_AUDIENCE: &str = "refresh-session";
const ISSUER: &str = "naoTimes/showtimes-rs";

/// The audience for the token, we use an enum to ensure we only use the correct values
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShowtimesAudience {
    /// Token is for user auth
    User,
    /// Token is for API key auth, this has no expiration
    #[serde(rename = "api-key")]
    APIKey,
    /// Master key auth, this has no expiration
    MasterKey,
    /// Token is for state jacking protection of Discord OAuth2
    DiscordAuth,
}

impl std::fmt::Display for ShowtimesAudience {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShowtimesAudience::User => write!(f, "user"),
            ShowtimesAudience::DiscordAuth => write!(f, "discord-auth"),
            ShowtimesAudience::APIKey => write!(f, "api-key"),
            ShowtimesAudience::MasterKey => write!(f, "master-key"),
        }
    }
}

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowtimesUserClaims {
    /// When the token expires
    exp: i64,
    /// When the token was issued
    #[serde(with = "jiff::fmt::serde::timestamp::second::required")]
    iat: jiff::Timestamp,
    /// Who issued the token, usually `showtimes-rs-session/{version}`
    iss: String,
    /// Who the token is for
    aud: ShowtimesAudience,
    /// Depending on the use case, this will be Ulid if it's a user token
    /// or a final redirect URL if it's a Discord OAuth2 state token
    metadata: String,
}

/// Our refresh claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowtimesRefreshClaims {
    /// When the token expires
    exp: i64,
    /// When the token was issued
    #[serde(with = "jiff::fmt::serde::timestamp::second::required")]
    iat: jiff::Timestamp,
    /// Who issued the token, usually `showtimes-rs-session/{version}`
    iss: String,
    /// Current user ID
    #[serde(with = "showtimes_shared::ulid_serializer")]
    user: showtimes_shared::ulid::Ulid,
    /// The audience, this is always RefreshTokenAudience
    aud: String,
}

impl ShowtimesUserClaims {
    fn new(id: showtimes_shared::ulid::Ulid, expires_in: i64) -> Self {
        let iat = jiff::Timestamp::now();
        let default = jiff::Timestamp::new(2_147_483_647, 0).unwrap();
        let exp = if expires_in == 0 {
            // Do a 32-bit max value
            default
        } else {
            let exp_in = jiff::SignedDuration::new(expires_in, 0);
            iat.saturating_add(exp_in).expect("This should not happens")
        };

        Self {
            exp: exp.as_second(),
            iat,
            iss: ISSUER.to_string(),
            aud: ShowtimesAudience::User,
            metadata: id.to_string(),
        }
    }

    /// Create a new API key claims
    pub fn new_api(api_key: String, aud: ShowtimesAudience) -> Self {
        let iat = jiff::Timestamp::now();
        Self {
            exp: -1i64,
            iat,
            iss: ISSUER.to_string(),
            aud,
            metadata: api_key,
        }
    }

    fn new_state(redirect_url: impl Into<String>) -> Self {
        let iat = jiff::Timestamp::now();
        // Discord OAuth2 request last 5 minutes
        let exp = iat
            .saturating_add(5.minutes())
            .expect("This should not happens");

        Self {
            exp: exp.as_second(),
            iat,
            iss: ISSUER.to_string(),
            aud: ShowtimesAudience::DiscordAuth,
            metadata: redirect_url.into(),
        }
    }

    /// Get metadata information, usually user ID
    pub fn get_metadata(&self) -> &str {
        &self.metadata
    }

    /// Get the expiry time in UNIX timestamp format
    pub fn get_expires_at(&self) -> i64 {
        self.exp
    }

    /// Get when the claims is issued
    pub fn get_issued_at(&self) -> jiff::Timestamp {
        self.iat
    }

    /// Get the issuer, usually showtimes-rs-session/{version}
    pub fn get_issuer(&self) -> &str {
        &self.iss
    }

    /// Get the audience or who is it for this claims is made
    pub fn get_audience(&self) -> ShowtimesAudience {
        self.aud
    }
}

impl ShowtimesRefreshClaims {
    fn new(user: showtimes_shared::ulid::Ulid) -> Self {
        let iat = jiff::Timestamp::now();
        // Refresh claims last for 90 days
        let exp = iat
            .to_zoned(jiff::tz::TimeZone::UTC)
            .saturating_add(90.days())
            .timestamp();

        Self {
            exp: exp.as_second(),
            iat,
            user,
            iss: ISSUER.to_string(),
            aud: REFRESH_AUDIENCE.to_string(),
        }
    }

    /// Get the user associated with this refresh token claims
    pub fn get_user(&self) -> showtimes_shared::ulid::Ulid {
        self.user
    }

    /// Get when the claims is issued
    pub fn get_issued_at(&self) -> jiff::Timestamp {
        self.iat
    }
}

/// A wrapper around the encoded token and the claims
#[derive(Debug, Serialize, Deserialize)]
pub struct ShowtimesUserSession {
    /// The encoded token
    token: String,
    /// The claims of the token
    claims: ShowtimesUserClaims,
}

impl ShowtimesUserSession {
    /// Create a new user session
    pub fn new(token: impl Into<String>, claims: ShowtimesUserClaims) -> Self {
        Self {
            token: token.into(),
            claims,
        }
    }

    /// Get the encoded token
    pub fn get_token(&self) -> &str {
        &self.token
    }

    /// Get the claims of the token
    pub fn get_claims(&self) -> &ShowtimesUserClaims {
        &self.claims
    }
}

/// A wrapper around the encoded refresh token and the refresh claims
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShowtimesRefreshSession {
    /// The encoded refresh token
    token: String,
    /// The claims of the refresh token
    claims: ShowtimesRefreshClaims,
}

impl ShowtimesRefreshSession {
    /// Create a new refresh session
    pub fn new(token: impl Into<String>, claims: ShowtimesRefreshClaims) -> Self {
        Self {
            token: token.into(),
            claims,
        }
    }

    /// Get the encoded refresh token
    pub fn get_token(&self) -> &str {
        &self.token
    }

    /// Get the claims of the refresh token
    pub fn get_claims(&self) -> &ShowtimesRefreshClaims {
        &self.claims
    }
}

/// Create a new session for the given user ID and expiration time.
///
/// The session will be signed with the provided secret key.
///
/// Returns a tuple containing the session information and the refresh token.
pub fn create_session(
    user_id: showtimes_shared::ulid::Ulid,
    expires_in: i64,
    secret: &jwt_lc_rs::Signer,
) -> Result<(ShowtimesUserSession, String), SessionError> {
    let user = ShowtimesUserClaims::new(user_id, expires_in);

    let token = jwt_lc_rs::encode(&user, secret)?;

    let session = ShowtimesUserSession::new(&token, user);
    let refresh_claims = ShowtimesRefreshClaims::new(user_id);

    let refresh_token = jwt_lc_rs::encode(&refresh_claims, secret)?;

    Ok((session, refresh_token))
}

/// Create a new API key session for the given API key and expiration time.
pub fn create_api_key_session(
    api_key: impl Into<String>,
    audience: ShowtimesAudience,
    secret: &jwt_lc_rs::Signer,
) -> Result<String, SessionError> {
    let audiences = VALID_API_AUDIENCES.clone();
    match audience {
        ShowtimesAudience::APIKey | ShowtimesAudience::MasterKey => {}
        _ => {
            return Err(SessionError::from(ValidationError::InvalidAudience(
                audiences,
            )));
        }
    }

    let user = ShowtimesUserClaims::new_api(api_key.into(), audience);

    jwt_lc_rs::encode(&user, secret)
}

/// Create a new Discord session state for the given redirect URL and secret.
pub fn create_discord_session_state(
    redirect_url: impl Into<String>,
    secret: &jwt_lc_rs::Signer,
) -> Result<String, SessionError> {
    let user = ShowtimesUserClaims::new_state(redirect_url);

    jwt_lc_rs::encode(&user, secret)
}

/// Verify an active JWT session token.
///
/// Return the claims if the token is valid and matches the expected audience.
/// Otherwise, return an error
pub fn verify_session(
    token: &str,
    secret: &jwt_lc_rs::Signer,
    expect_audience: ShowtimesAudience,
) -> Result<ShowtimesUserClaims, SessionError> {
    let validators: Vec<Box<dyn jwt_lc_rs::validator::Validation>> = vec![
        Box::new(jwt_lc_rs::validator::AudienceValidator::new(&[
            expect_audience,
        ])),
        Box::new(jwt_lc_rs::validator::IssuerValidator::new(&[ISSUER])),
        Box::new(jwt_lc_rs::validator::ExpiryValidator::new(2u64 * 60)), // 2 min grace
    ];

    jwt_lc_rs::decode(token, secret, &Validator::new(validators)).map(|data| data.into_claims())
}

/// Refresh a JWT session token.
pub fn refresh_session(
    token: &str,
    secret: &jwt_lc_rs::Signer,
    expires_in: i64,
) -> Result<ShowtimesUserSession, SessionError> {
    let validators: Vec<Box<dyn jwt_lc_rs::validator::Validation>> = vec![
        Box::new(jwt_lc_rs::validator::AudienceValidator::new(&[
            REFRESH_AUDIENCE,
        ])),
        Box::new(jwt_lc_rs::validator::IssuerValidator::new(&[ISSUER])),
        Box::new(jwt_lc_rs::validator::ExpiryValidator::new(30u64)), // 30s grace
    ];

    match jwt_lc_rs::decode::<ShowtimesRefreshClaims>(token, secret, &Validator::new(validators)) {
        Ok(data) => {
            let (session, _) = create_session(data.get_claims().get_user(), expires_in, secret)?;
            Ok(session)
        }
        Err(e) => Err(e),
    }
}

/// Verify a Refresh JWT session token.
pub(crate) fn verify_refresh_session(
    token: &str,
    secret: &jwt_lc_rs::Signer,
) -> Result<ShowtimesRefreshClaims, SessionError> {
    let validators: Vec<Box<dyn jwt_lc_rs::validator::Validation>> = vec![
        Box::new(jwt_lc_rs::validator::AudienceValidator::new(&[
            REFRESH_AUDIENCE,
        ])),
        Box::new(jwt_lc_rs::validator::IssuerValidator::new(&[ISSUER])),
        Box::new(jwt_lc_rs::validator::ExpiryValidator::new(30u64)), // 30s grace
    ];

    jwt_lc_rs::decode(token, secret, &Validator::new(validators)).map(|data| data.into_claims())
}

#[cfg(test)]
mod tests {
    use jwt_lc_rs::HmacAlgorithm;
    use showtimes_shared::ulid_serializer;

    use super::*;

    const REDIRECT_URL: &str = "/oauth2/test/discord";
    static SECRET_INFO: LazyLock<jwt_lc_rs::Signer> = LazyLock::new(|| {
        jwt_lc_rs::Signer::Hmac(HmacAlgorithm::new(
            jwt_lc_rs::SHALevel::SHA512,
            "super-duper-secret-for-testing",
        ))
    });

    #[test]
    fn test_valid_session() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let claims = verify_session(&token, &SECRET_INFO, ShowtimesAudience::User).unwrap();

        assert_eq!(claims.get_metadata(), &user_id.to_string());
        assert_eq!(claims.get_issuer(), ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::User);
    }

    #[test]
    fn test_valid_session_with_invalid_aud() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let result = verify_session(&token, &SECRET_INFO, ShowtimesAudience::DiscordAuth);

        match result {
            Err(e) => match e {
                SessionError::ValidationError(ValidationError::InvalidAudience(_)) => {
                    // Expected
                }
                _ => {
                    panic!("Expected an error of InvalidAudience, got {:?}", e);
                }
            },
            Ok(r) => panic!("Expected an error of InvalidAudience, got {:?}", r),
        }
    }

    #[test]
    fn test_valid_discord_session_state() {
        let token = create_discord_session_state(REDIRECT_URL, &SECRET_INFO).unwrap();

        let claims = verify_session(&token, &SECRET_INFO, ShowtimesAudience::DiscordAuth).unwrap();

        assert_eq!(claims.get_metadata(), REDIRECT_URL);
        assert_eq!(claims.get_issuer(), ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::DiscordAuth);
    }

    #[test]
    fn test_valid_discord_session_state_with_invalid_aud() {
        let token = create_discord_session_state(REDIRECT_URL, &SECRET_INFO).unwrap();

        let result = verify_session(&token, &SECRET_INFO, ShowtimesAudience::User);

        match result {
            Err(e) => match e {
                SessionError::ValidationError(ValidationError::InvalidAudience(_)) => {
                    // Expected
                }
                _ => {
                    panic!("Expected an error of InvalidAudience, got {:?}", e);
                }
            },
            Ok(r) => panic!("Expected an error of InvalidAudience, got {:?}", r),
        }
    }

    #[test]
    fn test_with_valid_header() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let header = jwt_lc_rs::decode_header(&token).unwrap();

        assert_eq!(header.alg, jwt_lc_rs::Algorithm::HS512);
    }
}

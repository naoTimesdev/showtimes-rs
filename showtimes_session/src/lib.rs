#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use std::sync::LazyLock;

use chrono::TimeZone;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use showtimes_shared::unix_timestamp_serializer;

pub mod manager;
pub mod oauth2;

/// Re-export the error type from the jsonwebtoken crate
pub use jsonwebtoken::errors::Error as SessionError;
/// Re-export the error kind from the jsonwebtoken crate
pub use jsonwebtoken::errors::ErrorKind as SessionErrorKind;

// The issuer of the token, we use a LazyLock to ensure it's only created once
static ISSUER: LazyLock<String> =
    LazyLock::new(|| format!("showtimes-rs-session/{}", env!("CARGO_PKG_VERSION")));
const REFRESH_AUDIENCE: &str = "refresh-session";

/// The algorithm we use for our tokens
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ShowtimesSHAMode {
    /// SHA-256
    SHA256,
    /// SHA-384
    #[default]
    SHA384,
    /// SHA-512, unavailable for ECDSA
    SHA512,
}

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

/// The encoding system used to sign/verify tokens
#[derive(Clone)]
pub struct ShowtimesEncodingKey {
    key: EncodingKey,
    decode_key: DecodingKey,
    header: Header,
}

impl std::fmt::Debug for ShowtimesEncodingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EncodingKey {{ header: {:?} }}", self.header)
    }
}

impl ShowtimesEncodingKey {
    /// Create a new encoding key in HMAC mode
    pub fn new_hmac(secret: impl Into<String>, mode: ShowtimesSHAMode) -> Self {
        let secret_str: String = secret.into();

        Self {
            key: EncodingKey::from_secret(secret_str.as_ref()),
            decode_key: DecodingKey::from_secret(secret_str.as_ref()),
            header: match mode {
                ShowtimesSHAMode::SHA256 => Header::new(Algorithm::HS256),
                ShowtimesSHAMode::SHA384 => Header::new(Algorithm::HS384),
                ShowtimesSHAMode::SHA512 => Header::new(Algorithm::HS512),
            },
        }
    }

    /// Create a new encoding key in RSA mode
    ///
    /// Only accept PEM encoded key.
    ///
    /// The accepted RSA should be generated with RSA-PSS/PKCS#1 v2.1 mode
    pub fn new_rsa(
        public: &[u8],
        private: &[u8],
        mode: ShowtimesSHAMode,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        Ok(Self {
            key: EncodingKey::from_rsa_pem(private)?,
            decode_key: DecodingKey::from_rsa_pem(public)?,
            header: match mode {
                ShowtimesSHAMode::SHA256 => Header::new(Algorithm::PS256),
                ShowtimesSHAMode::SHA384 => Header::new(Algorithm::PS384),
                ShowtimesSHAMode::SHA512 => Header::new(Algorithm::PS512),
            },
        })
    }

    /// Create a new encoding key in ECDSA mode
    ///
    /// Only accept PEM encoded key, this must be in PKCS#8 mode.
    ///
    /// Note: If you use SHA512 mode, this will automatically use SHA384
    pub fn new_ecdsa(
        public: &[u8],
        private: &[u8],
        mode: ShowtimesSHAMode,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        Ok(Self {
            key: EncodingKey::from_ec_pem(private)?,
            decode_key: DecodingKey::from_ec_pem(public)?,
            header: match mode {
                ShowtimesSHAMode::SHA256 => Header::new(Algorithm::ES256),
                ShowtimesSHAMode::SHA384 => Header::new(Algorithm::ES384),
                ShowtimesSHAMode::SHA512 => Header::new(Algorithm::ES384),
            },
        })
    }

    /// Do the actual encoding
    fn encode<T: Serialize>(&self, claims: &T) -> Result<String, jsonwebtoken::errors::Error> {
        jsonwebtoken::encode(&self.header, claims, &self.key)
    }

    /// Do the actual decoding
    fn decode<T: DeserializeOwned>(
        &self,
        token: &str,
        audience: impl std::fmt::Display,
    ) -> Result<jsonwebtoken::TokenData<T>, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(self.header.alg);
        validation.set_issuer(&[&*ISSUER]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["iat", "iss", "aud"]);
        jsonwebtoken::decode(token, &self.decode_key, &validation)
    }

    /// Test encode
    pub fn test_encode(&self) -> Result<String, jsonwebtoken::errors::Error> {
        let stub_user =
            ShowtimesUserClaims::new_api("test-api-key".to_string(), ShowtimesAudience::APIKey);
        self.encode(&stub_user)
    }

    /// Test decode
    pub fn test_decode(
        &self,
        token: &str,
    ) -> Result<jsonwebtoken::TokenData<ShowtimesUserClaims>, jsonwebtoken::errors::Error> {
        self.decode(token, ShowtimesAudience::APIKey)
    }
}

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowtimesUserClaims {
    /// When the token expires
    exp: i64,
    /// When the token was issued
    #[serde(with = "unix_timestamp_serializer")]
    iat: chrono::DateTime<chrono::Utc>,
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
    #[serde(with = "unix_timestamp_serializer")]
    iat: chrono::DateTime<chrono::Utc>,
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
        let iat = chrono::Utc::now();
        let exp = if expires_in == 0 {
            // Do a 32-bit max value
            chrono::Utc.timestamp_opt(2_147_483_647, 0).unwrap()
        } else {
            iat + chrono::Duration::seconds(expires_in)
        };

        Self {
            exp: exp.timestamp(),
            iat,
            iss: ISSUER.clone(),
            aud: ShowtimesAudience::User,
            metadata: id.to_string(),
        }
    }

    /// Create a new API key claims
    pub fn new_api(api_key: String, aud: ShowtimesAudience) -> Self {
        let iat = chrono::Utc::now();
        Self {
            exp: -1i64,
            iat,
            iss: ISSUER.clone(),
            aud,
            metadata: api_key,
        }
    }

    fn new_state(redirect_url: impl Into<String>) -> Self {
        let iat = chrono::Utc::now();
        // Discord OAuth2 request last 5 minutes
        let exp = iat + chrono::Duration::seconds(300);

        Self {
            exp: exp.timestamp(),
            iat,
            iss: ISSUER.clone(),
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
    pub fn get_issued_at(&self) -> chrono::DateTime<chrono::Utc> {
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
        let iat = chrono::Utc::now();
        // Refresh claims last for 90 days
        let exp = iat + chrono::Duration::days(90);

        Self {
            exp: exp.timestamp(),
            iat,
            user,
            iss: ISSUER.clone(),
            aud: REFRESH_AUDIENCE.to_string(),
        }
    }

    /// Get the user associated with this refresh token claims
    pub fn get_user(&self) -> showtimes_shared::ulid::Ulid {
        self.user
    }

    /// Get when the claims is issued
    pub fn get_issued_at(&self) -> chrono::DateTime<chrono::Utc> {
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
    secret: &ShowtimesEncodingKey,
) -> Result<(ShowtimesUserSession, String), SessionError> {
    let user = ShowtimesUserClaims::new(user_id, expires_in);

    let token = secret.encode(&user)?;

    let session = ShowtimesUserSession::new(&token, user);
    let refresh_claims = ShowtimesRefreshClaims::new(user_id);

    let refresh_token = secret.encode(&refresh_claims)?;

    Ok((session, refresh_token))
}

/// Create a new API key session for the given API key and expiration time.
pub fn create_api_key_session(
    api_key: impl Into<String>,
    audience: ShowtimesAudience,
    secret: &ShowtimesEncodingKey,
) -> Result<String, SessionError> {
    match audience {
        ShowtimesAudience::APIKey | ShowtimesAudience::MasterKey => {}
        _ => return Err(SessionError::from(SessionErrorKind::InvalidAudience)),
    }

    let user = ShowtimesUserClaims::new_api(api_key.into(), audience);

    secret.encode(&user)
}

/// Create a new Discord session state for the given redirect URL and secret.
pub fn create_discord_session_state(
    redirect_url: impl Into<String>,
    secret: &ShowtimesEncodingKey,
) -> Result<String, SessionError> {
    let user = ShowtimesUserClaims::new_state(redirect_url);

    secret.encode(&user)
}

/// Verify an active JWT session token.
///
/// Return the claims if the token is valid and matches the expected audience.
/// Otherwise, return an error
pub fn verify_session(
    token: &str,
    secret: &ShowtimesEncodingKey,
    expect_audience: ShowtimesAudience,
) -> Result<ShowtimesUserClaims, SessionError> {
    match secret.decode::<ShowtimesUserClaims>(token, expect_audience) {
        Ok(data) => {
            // 2 minutes allowance
            let current_time = chrono::Utc::now() - chrono::Duration::minutes(2);

            if data.claims.exp < current_time.timestamp() {
                Err(SessionError::from(SessionErrorKind::ExpiredSignature))
            } else {
                Ok(data.claims)
            }
        }
        Err(e) => Err(e),
    }
}

/// Refresh a JWT session token.
pub fn refresh_session(
    token: &str,
    secret: &ShowtimesEncodingKey,
    expires_in: i64,
) -> Result<ShowtimesUserSession, SessionError> {
    match secret.decode::<ShowtimesRefreshClaims>(token, REFRESH_AUDIENCE) {
        Ok(data) => {
            // Create session
            let (session, _) = create_session(data.claims.get_user(), expires_in, secret)?;
            Ok(session)
        }
        Err(e) => Err(e),
    }
}

/// Verify a Refresh JWT session token.
pub(crate) fn verify_refresh_session(
    token: &str,
    secret: &ShowtimesEncodingKey,
) -> Result<ShowtimesRefreshClaims, SessionError> {
    match secret.decode::<ShowtimesRefreshClaims>(token, REFRESH_AUDIENCE) {
        Ok(data) => Ok(data.claims),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use showtimes_shared::ulid_serializer;

    use super::*;

    const REDIRECT_URL: &str = "/oauth2/test/discord";
    static SECRET_INFO: LazyLock<ShowtimesEncodingKey> = LazyLock::new(|| {
        ShowtimesEncodingKey::new_hmac("super-duper-secret-for-testing", ShowtimesSHAMode::SHA512)
    });

    #[test]
    fn test_valid_session() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let claims = verify_session(&token, &SECRET_INFO, ShowtimesAudience::User).unwrap();

        assert_eq!(claims.get_metadata(), &user_id.to_string());
        assert_eq!(claims.get_issuer(), &*ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::User);
    }

    #[test]
    fn test_valid_session_with_invalid_aud() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let result = verify_session(&token, &SECRET_INFO, ShowtimesAudience::DiscordAuth);

        match result {
            Err(e) => {
                assert_eq!(e.kind(), &SessionErrorKind::InvalidAudience);
            }
            Ok(r) => panic!("Expected an error of InvalidAudience, got {:?}", r),
        }
    }

    #[test]
    fn test_valid_discord_session_state() {
        let token = create_discord_session_state(REDIRECT_URL, &SECRET_INFO).unwrap();

        let claims = verify_session(&token, &SECRET_INFO, ShowtimesAudience::DiscordAuth).unwrap();

        assert_eq!(claims.get_metadata(), REDIRECT_URL);
        assert_eq!(claims.get_issuer(), &*ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::DiscordAuth);
    }

    #[test]
    fn test_valid_discord_session_state_with_invalid_aud() {
        let token = create_discord_session_state(REDIRECT_URL, &SECRET_INFO).unwrap();

        let result = verify_session(&token, &SECRET_INFO, ShowtimesAudience::User);

        match result {
            Err(e) => {
                assert_eq!(e.kind(), &SessionErrorKind::InvalidAudience);
            }
            Ok(r) => panic!("Expected an error of InvalidAudience, got {:?}", r),
        }
    }

    #[test]
    fn test_with_valid_header() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        let expected = Header::new(Algorithm::HS512);

        assert_eq!(header, expected);
    }

    #[test]
    fn test_with_invalid_header() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, &SECRET_INFO).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        let expected = Header::new(Algorithm::HS256);

        assert_ne!(header, expected, "Expected headers to not match");
    }
}

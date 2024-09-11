use std::sync::LazyLock;

use chrono::TimeZone;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use showtimes_shared::unix_timestamp_serializer;

pub mod manager;
pub mod oauth2;

/// Re-export the error type from the jsonwebtoken crate
pub use jsonwebtoken::errors::Error as SessionError;
/// Re-export the error kind from the jsonwebtoken crate
pub use jsonwebtoken::errors::ErrorKind as SessionErrorKind;

// The issuer of the token, we use a LazyLock to ensure it's only created once
static ISSUER: LazyLock<String> = LazyLock::new(|| {
    let iss = format!("showtimes-rs-session/{}", env!("CARGO_PKG_VERSION"));

    iss
});
// The algorithm we use for our tokens
const ALGORITHM: Algorithm = Algorithm::HS512;

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

    pub fn get_metadata(&self) -> &str {
        &self.metadata
    }

    pub fn get_expires_at(&self) -> i64 {
        self.exp
    }

    pub fn get_issued_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.iat
    }

    pub fn get_issuer(&self) -> &str {
        &self.iss
    }

    pub fn get_audience(&self) -> ShowtimesAudience {
        self.aud
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

pub fn create_session(
    user_id: showtimes_shared::ulid::Ulid,
    expires_in: i64,
    secret: impl Into<String>,
) -> Result<(ShowtimesUserClaims, String), SessionError> {
    let user = ShowtimesUserClaims::new(user_id, expires_in);

    let header = Header::new(ALGORITHM);
    let secret_str: String = secret.into();
    let secret = EncodingKey::from_secret(secret_str.as_bytes());

    let token = jsonwebtoken::encode(&header, &user, &secret)?;

    Ok((user, token))
}

pub fn create_api_key_session(
    api_key: impl Into<String>,
    secret: impl Into<String>,
    audience: ShowtimesAudience,
) -> Result<String, SessionError> {
    match audience {
        ShowtimesAudience::APIKey | ShowtimesAudience::MasterKey => {}
        _ => return Err(SessionError::from(SessionErrorKind::InvalidAudience)),
    }

    let user = ShowtimesUserClaims::new_api(api_key.into(), audience);

    let header = Header::new(ALGORITHM);
    let secret_str: String = secret.into();
    let secret = EncodingKey::from_secret(secret_str.as_bytes());

    jsonwebtoken::encode(&header, &user, &secret)
}

pub fn create_discord_session_state(
    redirect_url: impl Into<String>,
    secret: impl Into<String>,
) -> Result<String, SessionError> {
    let user = ShowtimesUserClaims::new_state(redirect_url);

    let header = Header::new(ALGORITHM);
    let secret_str: String = secret.into();
    let secret = EncodingKey::from_secret(secret_str.as_bytes());

    jsonwebtoken::encode(&header, &user, &secret)
}

pub fn verify_session(
    token: &str,
    secret: impl Into<String>,
    expect_audience: ShowtimesAudience,
) -> Result<ShowtimesUserClaims, SessionError> {
    let secret_str: String = secret.into();

    let secret = DecodingKey::from_secret(secret_str.as_bytes());
    let mut validation = Validation::new(ALGORITHM);

    validation.set_issuer(&[&*ISSUER]);
    validation.set_audience(&[expect_audience]);
    validation.set_required_spec_claims(&["iat", "iss", "aud"]);

    // Verify `exp` if -1 then no expiration
    match jsonwebtoken::decode::<ShowtimesUserClaims>(token, &secret, &validation) {
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

#[cfg(test)]
mod tests {
    use showtimes_shared::ulid_serializer;

    use super::*;

    const SECRET: &str = "super-duper-secret-for-testing";
    const REDIRECT_URL: &str = "/oauth2/test/discord";

    #[test]
    fn test_valid_session() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, SECRET).unwrap();

        println!("{}", token);

        let claims = verify_session(&token, SECRET, ShowtimesAudience::User).unwrap();

        assert_eq!(claims.get_metadata(), &user_id.to_string());
        assert_eq!(claims.get_issuer(), &*ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::User);
    }

    #[test]
    fn test_valid_session_with_invalid_aud() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, SECRET).unwrap();

        let result = verify_session(&token, SECRET, ShowtimesAudience::DiscordAuth);

        match result {
            Err(e) => {
                assert_eq!(e.kind(), &SessionErrorKind::InvalidAudience);
            }
            Ok(r) => panic!("Expected an error of InvalidAudience, got {:?}", r),
        }
    }

    #[test]
    fn test_valid_discord_session_state() {
        let token = create_discord_session_state(REDIRECT_URL, SECRET).unwrap();

        let claims = verify_session(&token, SECRET, ShowtimesAudience::DiscordAuth).unwrap();

        assert_eq!(claims.get_metadata(), REDIRECT_URL);
        assert_eq!(claims.get_issuer(), &*ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::DiscordAuth);
    }

    #[test]
    fn test_valid_discord_session_state_with_invalid_aud() {
        let token = create_discord_session_state(REDIRECT_URL, SECRET).unwrap();

        let result = verify_session(&token, SECRET, ShowtimesAudience::User);

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
        let (_, token) = create_session(user_id, 3600, SECRET).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        let expected = Header::new(ALGORITHM);

        assert_eq!(header, expected);
    }

    #[test]
    fn test_with_invalid_header() {
        let user_id = ulid_serializer::default();
        let (_, token) = create_session(user_id, 3600, SECRET).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        let expected = Header::new(Algorithm::HS256);

        assert_ne!(header, expected, "Expected headers to not match");
    }
}

use std::sync::LazyLock;

use chrono::TimeZone;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use showtimes_shared::unix_timestamp_serializer;

/// Re-export the error type from the jsonwebtoken crate
pub use jsonwebtoken::errors::Error as SessionError;
/// Re-export the error kind from the jsonwebtoken crate
pub use jsonwebtoken::errors::ErrorKind as SessionErrorKind;

// The issuer of the token, we use a LazyLock to ensure it's only created once
const ISSUER: LazyLock<String> = LazyLock::new(|| {
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
    /// Token is for state jacking protection of Discord OAuth2
    DiscordAuth,
}

impl std::fmt::Display for ShowtimesAudience {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShowtimesAudience::User => write!(f, "user"),
            ShowtimesAudience::DiscordAuth => write!(f, "discord-auth"),
        }
    }
}

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize)]
pub struct ShowtimesUserClaims {
    /// When the token expires
    #[serde(with = "unix_timestamp_serializer")]
    exp: chrono::DateTime<chrono::Utc>,
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
    fn new(id: showtimes_shared::ulid::Ulid, expires_in: u64) -> Self {
        let iat = chrono::Utc::now();
        let exp = if expires_in == 0 {
            // Do a 32-bit max value
            chrono::Utc.timestamp_opt(2_147_483_647, 0).unwrap()
        } else {
            iat + chrono::Duration::seconds(expires_in as i64)
        };

        Self {
            exp,
            iat,
            iss: ISSUER.clone(),
            aud: ShowtimesAudience::User,
            metadata: id.to_string(),
        }
    }

    fn new_state(redirect_url: impl Into<String>) -> Self {
        let iat = chrono::Utc::now();
        // Discord OAuth2 request last 5 minutes
        let exp = iat + chrono::Duration::seconds(300);

        Self {
            exp,
            iat,
            iss: ISSUER.clone(),
            aud: ShowtimesAudience::DiscordAuth,
            metadata: redirect_url.into(),
        }
    }

    pub fn get_metadata(&self) -> &str {
        &self.metadata
    }

    pub fn get_expires_at(&self) -> chrono::DateTime<chrono::Utc> {
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

pub fn create_session(
    user_id: showtimes_shared::ulid::Ulid,
    expires_in: u64,
    secret: impl Into<String>,
) -> Result<String, SessionError> {
    let user = ShowtimesUserClaims::new(user_id, expires_in);

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
) -> Result<ShowtimesUserClaims, SessionError> {
    let secret_str: String = secret.into();

    let secret = DecodingKey::from_secret(secret_str.as_bytes());
    let mut validation = Validation::new(ALGORITHM);

    validation.set_issuer(&[&*ISSUER]);
    validation.set_audience(&[ShowtimesAudience::User]);
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud"]);

    match jsonwebtoken::decode::<ShowtimesUserClaims>(token, &secret, &validation) {
        Ok(data) => Ok(data.claims),
        Err(e) => Err(e),
    }
}

pub fn verify_discord_session_state(
    token: &str,
    secret: impl Into<String>,
) -> Result<ShowtimesUserClaims, SessionError> {
    let secret_str: String = secret.into();

    let secret = DecodingKey::from_secret(secret_str.as_bytes());
    let mut validation = Validation::new(ALGORITHM);

    validation.set_issuer(&[&*ISSUER]);
    validation.set_audience(&[ShowtimesAudience::DiscordAuth]);
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud"]);

    match jsonwebtoken::decode::<ShowtimesUserClaims>(token, &secret, &validation) {
        Ok(data) => Ok(data.claims),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use showtimes_shared::ulid_serializer;

    use super::*;

    const SECRET: &'static str = "super-duper-secret-for-testing";
    const REDIRECT_URL: &'static str = "/oauth2/test/discord";

    #[test]
    fn test_valid_session() {
        let user_id = ulid_serializer::default();
        let token = create_session(user_id, 3600, SECRET).unwrap();

        println!("{}", token);

        let claims = verify_session(&token, SECRET).unwrap();

        assert_eq!(claims.get_metadata(), &user_id.to_string());
        assert_eq!(claims.get_issuer(), &*ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::User);
    }

    #[test]
    fn test_valid_session_with_invalid_aud() {
        let user_id = ulid_serializer::default();
        let token = create_session(user_id, 3600, SECRET).unwrap();

        let result = verify_discord_session_state(&token, SECRET);

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

        let claims = verify_discord_session_state(&token, SECRET).unwrap();

        assert_eq!(claims.get_metadata(), REDIRECT_URL);
        assert_eq!(claims.get_issuer(), &*ISSUER);
        assert_eq!(claims.get_audience(), ShowtimesAudience::DiscordAuth);
    }

    #[test]
    fn test_valid_discord_session_state_with_invalid_aud() {
        let token = create_discord_session_state(REDIRECT_URL, SECRET).unwrap();

        let result = verify_session(&token, SECRET);

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
        let token = create_session(user_id, 3600, SECRET).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        let expected = Header::new(ALGORITHM);

        assert_eq!(header, expected);
    }

    #[test]
    fn test_with_invalid_header() {
        let user_id = ulid_serializer::default();
        let token = create_session(user_id, 3600, SECRET).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        let expected = Header::new(Algorithm::HS256);

        assert_ne!(header, expected, "Expected headers to not match");
    }
}

use std::sync::LazyLock;

use chrono::TimeZone;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use showtimes_shared::{ulid::Ulid, ulid_serializer, unix_timestamp_serializer};

/// Re-export the error type from the jsonwebtoken crate
pub use jsonwebtoken::errors::Error as SessionError;

// The issuer of the token, we use a LazyLock to ensure it's only created once
const ISSUER: LazyLock<String> = LazyLock::new(|| {
    let iss = format!("showtimes-rs-session/{}", env!("CARGO_PKG_VERSION"));

    iss
});
// The algorithm we use for our tokens
const ALGORITHM: Algorithm = Algorithm::HS512;

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize)]
pub struct ShowtimesUserClaims {
    #[serde(with = "ulid_serializer")]
    id: Ulid,
    #[serde(with = "unix_timestamp_serializer")]
    exp: chrono::DateTime<chrono::Utc>,
    #[serde(with = "unix_timestamp_serializer")]
    iat: chrono::DateTime<chrono::Utc>,
    iss: String,
}

impl ShowtimesUserClaims {
    fn new(id: Ulid, expires_in: u64) -> Self {
        let iat = chrono::Utc::now();
        let exp = if expires_in == 0 {
            // Do a 32-bit max value
            chrono::Utc.timestamp_opt(2_147_483_647, 0).unwrap()
        } else {
            iat + chrono::Duration::seconds(expires_in as i64)
        };

        let iss = &*ISSUER;

        Self {
            id,
            exp,
            iat,
            iss: iss.clone(),
        }
    }

    pub fn get_id(&self) -> Ulid {
        self.id
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
}

pub fn create_session(
    user_id: Ulid,
    expires_in: u64,
    secret: impl Into<String>,
) -> Result<String, SessionError> {
    let user = ShowtimesUserClaims::new(user_id, expires_in);

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

    let iss = &*ISSUER;

    validation.set_issuer(&[iss]);
    validation.set_required_spec_claims(&["exp", "iat", "iss"]);

    match jsonwebtoken::decode::<ShowtimesUserClaims>(token, &secret, &validation) {
        Ok(data) => Ok(data.claims),
        Err(e) => Err(e),
    }
}

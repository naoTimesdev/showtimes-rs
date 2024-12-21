#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use serde::{ser::SerializeSeq, Deserialize};
use uuid::{Timestamp, Uuid};

pub mod config;
pub use config::Config;

/// Re-exports of the [`ulid`] crate
pub use ulid;

const API_KEY_PREFIX: &str = "nsh_";

/// Generate v7 UUID for the current timestamp
///
/// # Examples
/// ```rust
/// use showtimes_shared::generate_uuid;
///
/// let uuid = generate_uuid();
/// println!("{}", uuid);
/// ```
pub fn generate_uuid() -> Uuid {
    let ts = Timestamp::now(uuid::timestamp::context::NoContext);
    Uuid::new_v7(ts)
}

/// Convert UUIDv7 to ULID
///
/// # Examples
/// ```rust
/// use showtimes_shared::{generate_uuid, uuid_to_ulid};
///
/// let uuid = generate_uuid();
/// let ulid = uuid_to_ulid(uuid);
///
/// println!("{}", ulid);
/// ```
pub fn uuid_to_ulid(uuid: Uuid) -> ulid::Ulid {
    ulid::Ulid::from_bytes(*uuid.as_bytes())
}

/// Convert ULID to UUIDv7
///
/// # Examples
/// ```rust
/// use showtimes_shared::{generate_uuid, ulid_to_uuid, uuid_to_ulid};
///
/// let uuid_act = generate_uuid();
/// let ulid = uuid_to_ulid(uuid_act);
/// let uuid = ulid_to_uuid(ulid);
///
/// assert_eq!(uuid_act, uuid);
/// ```
pub fn ulid_to_uuid(ulid: ulid::Ulid) -> Uuid {
    let bita = ulid.to_bytes();
    Uuid::from_bytes(bita)
}

/// An API key parse error
#[derive(Clone, Copy, Debug)]
pub enum APIKeyParseError {
    /// Failed to parse UUID
    InvalidUUID,
    /// Wrong format provided
    InvalidFormat,
    /// UUID is incomplete at part X
    IncompleteUUID(u8),
}

impl std::fmt::Display for APIKeyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            APIKeyParseError::InvalidFormat => write!(f, "Invalid API Key format"),
            APIKeyParseError::InvalidUUID => write!(f, "Invalid UUID"),
            APIKeyParseError::IncompleteUUID(pos) => {
                write!(f, "Invalid UUID, incomplete part {}", pos)
            }
        }
    }
}

impl std::error::Error for APIKeyParseError {}

/// An API key for authentication
///
/// This is a UUIDv4 with a prefix of `nsh_`
///
/// Internally, this is only a UUIDv4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct APIKey {
    inner: uuid::Uuid,
}

impl APIKey {
    /// Create a new API key
    pub fn new() -> Self {
        let inner = uuid::Uuid::new_v4();
        Self { inner }
    }

    /// Convert the internal UUID into an API key string
    pub fn as_api_key(&self) -> String {
        let inner_str = self.inner.to_string().replace("-", "");
        format!("{}{}", API_KEY_PREFIX, inner_str)
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.inner
    }

    /// Parse a string into an API key
    pub fn from_string(input: impl Into<String>) -> Result<Self, APIKeyParseError> {
        let input: String = input.into();
        if !input.starts_with(API_KEY_PREFIX) {
            return Err(APIKeyParseError::InvalidFormat);
        }

        let input: String = input.replace(API_KEY_PREFIX, "");
        // UUID dash is replaced with empty string, so we need to insert it back
        // ex: cd427fdabb04495688aa97422a3f0320
        //     cd427fda-bb04-4956-88aa-97422a3f0320
        let uuid_a = input.get(0..8).ok_or(APIKeyParseError::IncompleteUUID(0))?;
        let uuid_b = input
            .get(8..12)
            .ok_or(APIKeyParseError::IncompleteUUID(1))?;
        let uuid_c = input
            .get(12..16)
            .ok_or(APIKeyParseError::IncompleteUUID(2))?;
        let uuid_d = input
            .get(16..20)
            .ok_or(APIKeyParseError::IncompleteUUID(3))?;
        let uuid_e = input
            .get(20..32)
            .ok_or(APIKeyParseError::IncompleteUUID(4))?;
        let rfmt_s = format!("{}-{}-{}-{}-{}", uuid_a, uuid_b, uuid_c, uuid_d, uuid_e);

        let inner = uuid::Uuid::parse_str(&rfmt_s).map_err(|_| APIKeyParseError::InvalidUUID)?;
        Ok(APIKey { inner })
    }

    /// Parse a UUID into an API key
    pub fn from_uuid(input: uuid::Uuid) -> Self {
        Self { inner: input }
    }
}

impl std::fmt::Display for APIKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_api_key())
    }
}

impl Default for APIKey {
    fn default() -> Self {
        Self::new()
    }
}

// serde
impl serde::Serialize for APIKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_api_key())
    }
}

impl<'de> serde::Deserialize<'de> for APIKey {
    fn deserialize<D>(deserializer: D) -> Result<APIKey, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        APIKey::from_string(s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for APIKey {
    type Error = APIKeyParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        APIKey::from_string(value)
    }
}

impl TryFrom<&str> for APIKey {
    type Error = APIKeyParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        APIKey::from_string(value.to_string())
    }
}

/// A (de)serializer for [`ulid::Ulid`] for serde.
pub mod ulid_serializer {
    use super::*;

    /// Default value for [`ulid::Ulid`]
    ///
    /// Used for serde
    pub fn default() -> ulid::Ulid {
        uuid_to_ulid(generate_uuid())
    }

    /// Serialize [`ulid::Ulid`] to string
    ///
    /// Used for serde serialization
    pub fn serialize<S>(ulid: &ulid::Ulid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&ulid.to_string())
    }

    /// Deserialize string to [`ulid::Ulid`]
    ///
    /// Used for serde deserialization
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ulid::Ulid, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ulid::Ulid::from_string(&s).map_err(serde::de::Error::custom)
    }
}

/// A (de)serializer for [`ulid::Ulid`] for serde with optional support.
pub mod ulid_opt_serializer {
    use super::*;

    /// Serialize an optional [`ulid::Ulid`] to string
    ///
    /// Used for serde serialization
    pub fn serialize<S>(ulid: &Option<ulid::Ulid>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match ulid {
            Some(ulid) => serializer.serialize_str(&ulid.to_string()),
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize optional string to optional [`ulid::Ulid`]
    ///
    /// Used for serde deserialization
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<ulid::Ulid>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Option::<String>::deserialize(deserializer)?;
        match s {
            Some(s) => ulid::Ulid::from_string(&s)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// A (de)serializer for [`ulid::Ulid`] for serde with list or [`Vec`] support.
pub mod ulid_list_serializer {
    use super::*;

    /// Serialize a list of [`ulid::Ulid`] to a list of strings
    ///
    /// Used for serde serialization
    pub fn serialize<S>(ulids: &[ulid::Ulid], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(ulids.len()))?;
        for ulid in ulids {
            seq.serialize_element(&ulid.to_string())?;
        }
        seq.end()
    }

    /// Deserialize list of strings to list of [`ulid::Ulid`]
    ///
    /// Used for serde deserialization
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<ulid::Ulid>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Vec::<String>::deserialize(deserializer)?;
        s.iter()
            .map(|s| ulid::Ulid::from_string(s).map_err(serde::de::Error::custom))
            .collect()
    }
}

/// A (de)serializer for [`chrono::Utc`] of [`chrono::DateTime`] to be converted to BSON-compatible data.
pub mod bson_datetime_opt_serializer {
    use super::*;
    use bson::DateTime;
    use chrono::Utc;
    use serde::Serialize;

    /// De-serialize a BSON-compatible datetime to a [`chrono::Utc`] of [`chrono::DateTime`].
    ///
    /// Used for serde deserialization
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<chrono::DateTime<Utc>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match DateTime::deserialize(deserializer) {
            Ok(dt) => {
                let dt = dt.to_chrono();
                Ok(Some(dt))
            }
            Err(_) => Ok(None),
        }
    }

    /// Serialize a [`chrono::Utc`] of [`chrono::DateTime`] to a BSON-compatible datetime.
    ///
    /// Used for serde serialization
    pub fn serialize<S>(
        date: &Option<chrono::DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match date {
            Some(date) => {
                let dt = DateTime::from_chrono(*date);
                dt.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }
}

/// A (de)serializer for [`chrono::Utc`] of [`chrono::DateTime`] to be converted to UNIX timestamp format.
pub mod unix_timestamp_serializer {
    use serde::Deserialize;

    /// Serialize a [`chrono::Utc`] of [`chrono::DateTime`] to a UNIX timestamp ([`i64`]).
    ///
    /// Used for serde serialization
    pub fn serialize<S>(
        date: &chrono::DateTime<chrono::Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ts = date.timestamp();
        serializer.serialize_i64(ts)
    }

    /// Deserialize a UNIX timestamp ([`i64`]) to a [`chrono::Utc`] of [`chrono::DateTime`].
    ///
    /// Used for serde deserialization
    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ts = i64::deserialize(deserializer)?;
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid timestamp: {}", ts)))?;
        Ok(dt)
    }
}

/// A (de)serializer for [`chrono::Utc`] of [`chrono::DateTime`]
/// to be converted to UNIX timestamp with optional support.
pub mod unix_timestamp_opt_serializer {
    use serde::Deserialize;

    /// Serialize a plausible [`chrono::Utc`] of [`chrono::DateTime`] to a UNIX timestamp ([`i64`]).
    ///
    /// Used for serde serialization
    pub fn serialize<S>(
        date: &Option<chrono::DateTime<chrono::Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match date {
            Some(date) => {
                let ts = date.timestamp();
                serializer.serialize_i64(ts)
            }
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize a plausible UNIX timestamp ([`i64`]) to a [`chrono::Utc`] of [`chrono::DateTime`].
    ///
    /// Used for serde deserialization
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match i64::deserialize(deserializer) {
            Ok(s) => {
                // unwrap now!
                match chrono::DateTime::<chrono::Utc>::from_timestamp(s, 0) {
                    Some(dt) => Ok(Some(dt)),
                    None => Err(serde::de::Error::custom(format!(
                        "invalid timestamp: {}",
                        s
                    ))),
                }
            }
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uuid() {
        let uuid = generate_uuid();
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
    }

    #[test]
    fn test_ulid_string() {
        let ts = Timestamp::from_unix(uuid::timestamp::context::NoContext, 1718276973, 0);
        let uuid_act = Uuid::new_v7(ts);

        let ulid = uuid_to_ulid(uuid_act);
        let uuid = ulid_to_uuid(ulid);

        println!("{:?}", uuid_act);
        println!("{:?}", ulid);
        println!("{:?}", uuid);

        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
        assert_eq!(uuid.get_timestamp(), Some(ts));
        assert_eq!(uuid_act, uuid);
    }

    #[test]
    fn parse_api_key() {
        let api_key = "nsh_cd427fdabb04495688aa97422a3f0320";
        let expect_key = uuid::Uuid::parse_str("cd427fda-bb04-4956-88aa-97422a3f0320").unwrap();
        let api_key = APIKey::from_string(api_key).unwrap();

        assert_eq!(api_key.as_uuid().get_version(), Some(uuid::Version::Random));
        assert_eq!(api_key.as_uuid(), expect_key);
    }

    #[test]
    fn test_api_key_fails() {
        let api_key = "shn_cd427fdabb04495688aa97422a3f0320";
        let api_key = APIKey::from_string(api_key);

        assert!(api_key.is_err(), "Invalid API key format (expect nsh_)");
    }

    #[test]
    fn test_api_key_fails_uuid() {
        let api_key = "nsh_cd427fdabb04495688aa97422a3f03";

        let api_key = APIKey::from_string(api_key);
        assert!(api_key.is_err(), "Invalid UUID");
    }
}

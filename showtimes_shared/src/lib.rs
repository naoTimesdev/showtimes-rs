//! Shared utilities for Showtimes

use serde::Deserialize;
use uuid::{Timestamp, Uuid};

/// Re-exports of the [`ulid`] crate
pub use ulid;

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

/// Default value for [`ulid::Ulid`]
///
/// Used for serde
pub fn def_ulid() -> ulid::Ulid {
    uuid_to_ulid(generate_uuid())
}

/// Serialize [`ulid::Ulid`] to string
///
/// Used for serde serialization
pub fn ser_ulid<S>(ulid: &ulid::Ulid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&ulid.to_string())
}

/// Serialize an optional [`ulid::Ulid`] to string
///
/// Used for serde serialization
pub fn ser_opt_ulid<S>(ulid: &Option<ulid::Ulid>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match ulid {
        Some(ulid) => serializer.serialize_str(&ulid.to_string()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize string to [`ulid::Ulid`]
///
/// Used for serde deserialization
pub fn de_ulid<'de, D>(deserializer: D) -> Result<ulid::Ulid, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    ulid::Ulid::from_string(&s).map_err(serde::de::Error::custom)
}

/// Deserialize optional string to optional [`ulid::Ulid`]
///
/// Used for serde deserialization
pub fn de_opt_ulid<'de, D>(deserializer: D) -> Result<Option<ulid::Ulid>, D::Error>
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

pub fn ser_opt_unix<S>(
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

pub fn de_opt_unix<'de, D>(
    deserializer: D,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match i64::deserialize(deserializer) {
        Ok(s) => {
            // unwrap now!
            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(s, 0).unwrap();
            Ok(Some(dt))
        }
        Err(_) => Ok(None),
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
}

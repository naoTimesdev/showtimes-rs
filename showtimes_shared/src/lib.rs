//! Shared utilities for Showtimes

use serde::{ser::SerializeSeq, Deserialize};
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

/// A prefixed ULID
#[derive(Debug, Clone)]
pub struct PrefixUlid {
    prefix: String,
    ulid: ulid::Ulid,
}

impl PrefixUlid {
    /// Create a new prefixed ULID
    pub fn new(prefix: impl Into<String>) -> anyhow::Result<Self> {
        let prefix = prefix.into();
        Self::verify_prefix(&prefix)?;
        Ok(Self {
            prefix,
            ulid: ulid_serializer::default(),
        })
    }

    /// Create a new prefixed ULID with a specific ULID
    pub fn with_ulid(prefix: impl Into<String>, ulid: ulid::Ulid) -> anyhow::Result<Self> {
        let prefix = prefix.into();
        Self::verify_prefix(&prefix)?;
        Ok(Self { prefix, ulid })
    }

    fn verify_prefix(prefix: &str) -> anyhow::Result<()> {
        if prefix.is_empty() {
            anyhow::bail!("`prefix` cannot be empty");
        }
        if !prefix.is_ascii() {
            anyhow::bail!("`prefix` must be ASCII");
        }
        if prefix.len() > 10 {
            anyhow::bail!("`prefix` cannot be more than 10 characters");
        }
        // do not allow space and dash
        if prefix.contains(' ') || prefix.contains('-') {
            anyhow::bail!("`prefix` cannot contain space or dash");
        }
        Ok(())
    }
}

impl std::fmt::Display for PrefixUlid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.prefix, self.ulid)
    }
}

// serde
impl serde::Serialize for PrefixUlid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for PrefixUlid {
    fn deserialize<D>(deserializer: D) -> Result<PrefixUlid, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom("Invalid ULID"));
        }
        let prefix = parts[0];
        let ulid = parts[1];
        let ulid = ulid::Ulid::from_string(ulid).map_err(serde::de::Error::custom)?;
        PrefixUlid::with_ulid(prefix, ulid).map_err(serde::de::Error::custom)
    }
}

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

pub mod bson_datetime_opt_serializer {
    use super::*;
    use bson::DateTime;
    use chrono::Utc;
    use serde::Serialize;

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

pub mod unix_timestamp_serializer {
    use serde::Deserialize;

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

    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ts = i64::deserialize(deserializer)?;
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0).unwrap();
        Ok(dt)
    }
}

pub mod unix_timestamp_opt_serializer {
    use serde::Deserialize;

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

    pub fn deserialize<'de, D>(
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

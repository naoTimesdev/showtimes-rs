use jiff::Timestamp;

use serde::{de::Error as _, ser::Error as _};
use serde::{
    de::{Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};

pub fn serialize<S>(dt: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ts = dt.as_second();

    u32::try_from(ts)
        .map_err(|_| S::Error::custom(format!("{dt} cannot be represented as DateTime")))?
        .serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
where
    D: Deserializer<'de>,
{
    let ts: u32 = Deserialize::deserialize(deserializer)?;
    Timestamp::from_second(i64::from(ts)).map_err(D::Error::custom)
}

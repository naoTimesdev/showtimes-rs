use clickhouse::Row;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Debug;

/// [`EventKind`] represents the kind of event that can be published
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// User created event
    UserCreated = 1,
    /// User updated event
    UserUpdated = 2,
    /// User deleted event
    UserDeleted = 3,
    /// Server created event
    ServerCreated = 10,
    /// Server updated event
    ServerUpdated = 11,
    /// Server deleted event
    ServerDeleted = 12,
    /// Project created event
    ProjectCreated = 20,
    /// Project updated event
    ProjectUpdated = 21,
    /// Project deleted event
    ProjectDeleted = 22,
    /// Project episodes event, this is used to publish information
    /// changes for a single episode of a project.
    ProjectEpisodes = 30,
    /// Collaboration created event
    CollaborationCreated = 40,
    /// Collaboration accepted event
    CollaborationAccepted = 41,
    /// Collaboration rejected event
    CollaborationRejected = 42,
    /// Collaboration deleted event
    CollaborationDeleted = 43,
    /// Collaboration retracted event, used when the initiator cancels
    CollaborationRetracted = 44,
}

/// The event structure that is broadcasted and stored
#[derive(Clone, Debug, Row, Serialize, Deserialize)]
pub struct SHEvent<T: Send + Sync + Clone> {
    /// The ID of the event, this is randomly generated
    #[serde(
        deserialize_with = "deserialize_ulid",
        serialize_with = "serialize_ulid"
    )]
    id: showtimes_shared::ulid::Ulid,
    /// The event kind
    kind: EventKind,
    /// The event data itself, on Clickhouse this will be stored as a
    #[serde(
        bound(
            deserialize = "T: DeserializeOwned + Send + Sync + Clone + Debug",
            serialize = "T: Serialize + Send + Sync + Clone + Debug"
        ),
        deserialize_with = "deserialize_event_data",
        serialize_with = "serialize_event_data"
    )]
    data: T,
    /// The actor or the person who initiated the event
    ///
    /// If the event is initiated by the system/Owner, this will be `None`/null
    actor: Option<String>,
    /// The timestamp of the event
    #[serde(with = "clickhouse::serde::time::datetime")]
    timestamp: ::time::OffsetDateTime,
}

impl<T> SHEvent<T>
where
    T: serde::Serialize + Send + Sync + Clone + 'static,
{
    pub(crate) fn new(kind: EventKind, data: T) -> Self {
        Self {
            id: showtimes_shared::ulid_serializer::default(),
            kind,
            data,
            actor: None,
            timestamp: ::time::OffsetDateTime::now_utc(),
        }
    }

    pub(crate) fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn id(&self) -> showtimes_shared::ulid::Ulid {
        self.id
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }

    pub fn timestamp(&self) -> ::time::OffsetDateTime {
        self.timestamp
    }
}

fn deserialize_event_data<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: DeserializeOwned + Send + Sync + Clone,
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    let unwrap_data: T = serde_json::from_str(&s).map_err(serde::de::Error::custom)?;
    Ok(unwrap_data)
}

fn serialize_event_data<T, S>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize + Send + Sync + Clone,
    S: serde::Serializer,
{
    let s = serde_json::to_string(data).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&s)
}

fn serialize_ulid<S>(ulid: &showtimes_shared::ulid::Ulid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let to_uuid = showtimes_shared::ulid_to_uuid(*ulid);
    clickhouse::serde::uuid::serialize(&to_uuid, serializer)
}

fn deserialize_ulid<'de, D>(deserializer: D) -> Result<showtimes_shared::ulid::Ulid, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let uuid = clickhouse::serde::uuid::deserialize(deserializer)?;
    if uuid.get_version_num() != 7 {
        return Err(serde::de::Error::custom(format!(
            "Invalid UUID version, expected UUIDv7 got {}",
            uuid.get_version_num()
        )));
    }
    Ok(showtimes_shared::uuid_to_ulid(uuid))
}

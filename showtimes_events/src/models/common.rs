use clickhouse::Row;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

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

#[derive(Debug, Row, Serialize, Deserialize)]
pub(crate) struct SHEvent<T: Send + Sync + Clone> {
    /// The ID of the event, this is randomly generated
    #[serde(with = "clickhouse::serde::uuid")]
    id: uuid::Uuid,
    /// The event kind
    kind: EventKind,
    /// The event data itself, on Clickhouse this will be stored as a
    #[serde(
        bound(
            deserialize = "T: DeserializeOwned + Send + Sync + Clone",
            serialize = "T: Serialize + Send + Sync + Clone"
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
    pub fn new(kind: EventKind, data: T) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            kind,
            data,
            actor: None,
            timestamp: ::time::OffsetDateTime::now_utc(),
        }
    }

    pub fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
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

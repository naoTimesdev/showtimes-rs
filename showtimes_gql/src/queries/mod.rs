pub mod projects;
pub mod servers;
pub mod users;

/// The config for querying servers
#[derive(Debug, Clone, Copy)]
pub struct ServerQueryUser {
    /// The user ID
    id: showtimes_shared::ulid::Ulid,
    /// The user kind
    kind: showtimes_db::m::UserKind,
}

impl ServerQueryUser {
    /// Create a new server query user
    pub fn new(id: showtimes_shared::ulid::Ulid, kind: showtimes_db::m::UserKind) -> Self {
        ServerQueryUser { id, kind }
    }

    /// Get the user ID
    pub fn id(&self) -> showtimes_shared::ulid::Ulid {
        self.id
    }

    /// Get the user kind
    pub fn kind(&self) -> showtimes_db::m::UserKind {
        self.kind
    }
}

impl From<&showtimes_db::m::User> for ServerQueryUser {
    fn from(user: &showtimes_db::m::User) -> Self {
        ServerQueryUser::new(user.id, user.kind)
    }
}

impl From<showtimes_db::m::User> for ServerQueryUser {
    fn from(user: showtimes_db::m::User) -> Self {
        ServerQueryUser::new(user.id, user.kind)
    }
}

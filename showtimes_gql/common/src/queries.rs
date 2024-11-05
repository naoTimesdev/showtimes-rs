//! Some common queries models and functions

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

/// A minimal server information
#[derive(Debug, Clone)]
pub struct MinimalServerUsers {
    /// Server ID
    id: showtimes_shared::ulid::Ulid,
    /// Server users
    owners: Vec<showtimes_db::m::ServerUser>,
}

impl MinimalServerUsers {
    /// Create a new minimal server users
    pub fn new(id: showtimes_shared::ulid::Ulid, owners: Vec<showtimes_db::m::ServerUser>) -> Self {
        MinimalServerUsers { id, owners }
    }

    /// Get the current ID
    pub fn id(&self) -> showtimes_shared::ulid::Ulid {
        self.id
    }

    /// Get the current owners
    pub fn owners(&self) -> &[showtimes_db::m::ServerUser] {
        &self.owners
    }
}

impl From<showtimes_db::m::Server> for MinimalServerUsers {
    fn from(server: showtimes_db::m::Server) -> Self {
        MinimalServerUsers {
            id: server.id,
            owners: server.owners,
        }
    }
}

impl From<&showtimes_db::m::Server> for MinimalServerUsers {
    fn from(server: &showtimes_db::m::Server) -> Self {
        MinimalServerUsers {
            id: server.id,
            owners: server.owners.clone(),
        }
    }
}

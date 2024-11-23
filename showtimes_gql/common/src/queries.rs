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

    /// Convert the information of this query into a [`async_graphql::Value`].
    ///
    /// Mainly used as extra context for the [`async_graphql::ErrorExtensionValues`].
    pub fn as_graphql_value(&self) -> async_graphql::Value {
        let mut data =
            async_graphql::indexmap::IndexMap::<async_graphql::Name, async_graphql::Value>::new();
        data.entry(async_graphql::Name::new("id"))
            .or_insert(async_graphql::Value::String(self.id.to_string()));
        data.entry(async_graphql::Name::new("kind"))
            .or_insert(async_graphql::Value::String(
                self.kind.to_name().to_string(),
            ));

        async_graphql::Value::Object(data)
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

    /// Convert the information of this query into a [`async_graphql::Value`].
    ///
    /// Mainly used as extra context for the [`async_graphql::ErrorExtensionValues`].
    pub fn as_graphql_value(&self) -> async_graphql::Value {
        let mut data =
            async_graphql::indexmap::IndexMap::<async_graphql::Name, async_graphql::Value>::new();
        data.entry(async_graphql::Name::new("id"))
            .or_insert(async_graphql::Value::String(self.id.to_string()));
        let owners_mapped = self
            .owners
            .iter()
            .map(|owner| {
                let mut data_owner = async_graphql::indexmap::IndexMap::<
                    async_graphql::Name,
                    async_graphql::Value,
                >::new();
                data_owner
                    .entry(async_graphql::Name::new("id"))
                    .or_insert(async_graphql::Value::String(owner.id.to_string()));
                data_owner
                    .entry(async_graphql::Name::new("privilege"))
                    .or_insert(async_graphql::Value::String(
                        owner.privilege.to_name().to_string(),
                    ));
                let extra_maps = owner
                    .extras
                    .iter()
                    .map(|extra| async_graphql::Value::String(extra.to_string()))
                    .collect::<Vec<_>>();
                data_owner
                    .entry(async_graphql::Name::new("extras"))
                    .or_insert(async_graphql::Value::List(extra_maps));

                async_graphql::Value::Object(data_owner)
            })
            .collect::<Vec<_>>();
        data.entry(async_graphql::Name::new("owners"))
            .or_insert(async_graphql::Value::List(owners_mapped));

        async_graphql::Value::Object(data)
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

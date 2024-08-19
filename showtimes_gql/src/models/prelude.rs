use std::ops::Deref;

use async_graphql::{ComplexObject, Enum, Scalar, ScalarType, SimpleObject};

/// A wrapper around ULID to allow it to be used in GraphQL
pub struct UlidGQL(showtimes_shared::ulid::Ulid);

impl Deref for UlidGQL {
    type Target = showtimes_shared::ulid::Ulid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[Scalar]
impl ScalarType for UlidGQL {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => Ok(UlidGQL(s.parse()?)),
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

impl From<showtimes_shared::ulid::Ulid> for UlidGQL {
    fn from(ulid: showtimes_shared::ulid::Ulid) -> Self {
        UlidGQL(ulid)
    }
}

impl From<&showtimes_shared::ulid::Ulid> for UlidGQL {
    fn from(ulid: &showtimes_shared::ulid::Ulid) -> Self {
        UlidGQL(ulid.clone())
    }
}

/// A wrapper around DateTime<Utc> to allow it to be used in GraphQL
pub struct DateTimeGQL(chrono::DateTime<chrono::Utc>);

impl Deref for DateTimeGQL {
    type Target = chrono::DateTime<chrono::Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[Scalar]
impl ScalarType for DateTimeGQL {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => {
                let rfc3399 = chrono::DateTime::parse_from_rfc3339(&s)?;
                let utc = rfc3399.with_timezone(&chrono::Utc);

                Ok(DateTimeGQL(utc))
            }
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_rfc3339())
    }
}

impl From<chrono::DateTime<chrono::Utc>> for DateTimeGQL {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        DateTimeGQL(dt)
    }
}

impl From<&chrono::DateTime<chrono::Utc>> for DateTimeGQL {
    fn from(dt: &chrono::DateTime<chrono::Utc>) -> Self {
        DateTimeGQL(dt.clone())
    }
}

/// Enum to hold user kinds
#[derive(Enum, Default, Copy, Clone, Eq, PartialEq)]
#[graphql(remote = "showtimes_db::m::UserKind")]
pub enum UserKindGQL {
    /// A normal user
    #[default]
    User,
    /// An admin user, can see all users and manage all servers
    Admin,
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct ImageMetadataGQL {
    /// The type of the image
    kind: String,
    /// The key of the image (usually the project ID)
    key: String,
    /// The filename of the image
    filename: String,
    /// The format of the image
    format: String,
    /// The parent of the image (usually the server ID)
    parent: Option<String>,
}

#[ComplexObject]
impl ImageMetadataGQL {
    /// Get the full URL of the image without the host
    async fn url(&self) -> String {
        match &self.parent {
            Some(parent) => format!(
                "/{}/{}/{}/{}",
                &self.kind, parent, &self.key, &self.filename
            ),
            None => format!("/{}/{}/{}", &self.kind, &self.key, &self.filename),
        }
    }
}

impl From<showtimes_db::m::ImageMetadata> for ImageMetadataGQL {
    fn from(meta: showtimes_db::m::ImageMetadata) -> Self {
        ImageMetadataGQL {
            kind: meta.kind,
            key: meta.key,
            filename: meta.filename,
            format: meta.format,
            parent: meta.parent,
        }
    }
}

impl From<&showtimes_db::m::ImageMetadata> for ImageMetadataGQL {
    fn from(meta: &showtimes_db::m::ImageMetadata) -> Self {
        ImageMetadataGQL {
            kind: meta.kind.clone(),
            key: meta.key.clone(),
            filename: meta.filename.clone(),
            format: meta.format.clone(),
            parent: meta.parent.clone(),
        }
    }
}

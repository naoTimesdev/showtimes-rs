use std::ops::Deref;

use async_graphql::{ComplexObject, Description, OutputType, Scalar, ScalarType, SimpleObject};

use super::{projects::ProjectGQL, servers::ServerGQL};

/// A wrapper around ULID to allow it to be used in GraphQL
pub struct UlidGQL(showtimes_shared::ulid::Ulid);

impl Deref for UlidGQL {
    type Target = showtimes_shared::ulid::Ulid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Description for UlidGQL {
    fn description() -> &'static str {
        "A ULID (Universally Unique Lexicographically Sortable Identifier) used to uniquely identify objects\nThe following ULID are converted from UUID timestamp or UUIDv7 before being converted to a ULID"
    }
}

#[Scalar(use_type_description = true)]
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
pub struct DateTimeGQL(
    /// A datetime timestamp format in UTC timezone, follows RFC3339 format
    chrono::DateTime<chrono::Utc>,
);

impl Description for DateTimeGQL {
    fn description() -> &'static str {
        "A datetime timestamp format in UTC timezone, follows RFC3339 format"
    }
}

impl Deref for DateTimeGQL {
    type Target = chrono::DateTime<chrono::Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[Scalar(use_type_description = true)]
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

/// Information about an image
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

/// A page information for pagination
#[derive(SimpleObject)]
pub struct PageInfoGQL {
    /// The total number of pages
    total: u64,
    /// The number of items per page
    #[graphql(name = "perPage")]
    per_page: u32,
    /// Next cursor to get the next page
    #[graphql(name = "nextCursor")]
    next_cursor: Option<UlidGQL>,
}

/// A paginated data structure
#[derive(SimpleObject)]
#[graphql(concrete(name = "ProjectPaginated", params(ProjectGQL)))]
#[graphql(concrete(name = "ServerPaginated", params(ServerGQL)))]
pub struct PaginatedGQL<T: OutputType> {
    /// The items list
    node: Vec<T>,
    /// The page information
    #[graphql(name = "pageInfo")]
    page_info: PageInfoGQL,
}

impl PageInfoGQL {
    /// Create a new PageInfoGQL
    pub fn new(total: u64, per_page: u32, next_cursor: Option<UlidGQL>) -> Self {
        PageInfoGQL {
            total,
            per_page,
            next_cursor,
        }
    }

    /// Empty PageInfoGQL
    pub fn empty(per_page: u32) -> Self {
        PageInfoGQL {
            total: 0,
            per_page: per_page,
            next_cursor: None,
        }
    }
}

impl<T: OutputType> PaginatedGQL<T> {
    /// Create a new PaginatedGQL
    pub fn new(node: Vec<T>, page_info: PageInfoGQL) -> Self {
        PaginatedGQL { node, page_info }
    }
}

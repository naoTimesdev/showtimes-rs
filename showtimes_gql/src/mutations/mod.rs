use async_graphql::{CustomValidator, Enum, InputObject};

use crate::models::prelude::IntegrationTypeGQL;

pub mod collaborations;
pub mod projects;
pub mod servers;
pub mod users;

/// The list of possible integrations actions.
#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationActionGQL {
    /// Add
    Add,
    /// Update
    Update,
    /// Remove an integration
    Remove,
}

/// An integration input object on what to update
///
/// All fields are required, and the following restrictions are applied:
/// - `originalId` is required for `UPDATE` action
/// - When removing and no ID is found, it will be ignored
/// - When adding, if the ID is found, it will be ignored
#[derive(InputObject)]
pub struct IntegrationInputGQL {
    /// The integration ID or name
    id: String,
    /// Original ID of the integration
    ///
    /// Used only for `UPDATE` action
    #[graphql(name = "originalId")]
    original_id: Option<String>,
    /// The integration type
    kind: IntegrationTypeGQL,
    /// The integration action
    action: IntegrationActionGQL,
}

impl IntegrationInputGQL {
    /// Check if valid
    pub fn raise_if_invalid(
        &self,
    ) -> Result<(), async_graphql::InputValueError<IntegrationInputGQL>> {
        if self.action == IntegrationActionGQL::Update && self.original_id.is_none() {
            Err(
                async_graphql::InputValueError::custom("originalId is required for UPDATE action")
                    .with_extension("field", "originalId")
                    .with_extension("id", self.id.clone())
                    .with_extension("kind", self.kind),
            )
        } else {
            Ok(())
        }
    }
}

/// A simple validator for the integration input
pub struct IntegrationValidator {
    limit: Vec<IntegrationActionGQL>,
}

impl IntegrationValidator {
    /// Create a new integration validator
    pub fn new() -> Self {
        Self {
            limit: vec![
                IntegrationActionGQL::Add,
                IntegrationActionGQL::Update,
                IntegrationActionGQL::Remove,
            ],
        }
    }

    /// Create a new integration validator with a custom limit
    pub fn with_limit(limit: Vec<IntegrationActionGQL>) -> Self {
        Self { limit }
    }
}

impl CustomValidator<Vec<IntegrationInputGQL>> for IntegrationValidator {
    fn check(
        &self,
        value: &Vec<IntegrationInputGQL>,
    ) -> Result<(), async_graphql::InputValueError<Vec<IntegrationInputGQL>>> {
        for (idx, vs) in value.iter().enumerate() {
            vs.raise_if_invalid().map_err(|_| {
                async_graphql::InputValueError::custom("One or more integrations are invalid")
                    .with_extension("index", idx)
                    .with_extension("error", "missing field(s)")
                    .with_extension("kind", vs.kind)
                    .with_extension("action", vs.action)
                    .with_extension("id", vs.id.clone())
            })?;

            // Check if the action is valid
            if !self.limit.contains(&vs.action) {
                return Err(async_graphql::InputValueError::custom(
                    "Action not allowed for this input field",
                )
                .with_extension("index", idx)
                .with_extension("id", vs.id.clone())
                .with_extension("action", vs.action));
            }
        }

        Ok(())
    }
}

impl CustomValidator<IntegrationInputGQL> for IntegrationValidator {
    fn check(
        &self,
        value: &IntegrationInputGQL,
    ) -> Result<(), async_graphql::InputValueError<IntegrationInputGQL>> {
        value.raise_if_invalid()?;

        // Check if the action is valid
        if !self.limit.contains(&value.action) {
            return Err(async_graphql::InputValueError::custom(
                "Action not allowed for this input field",
            )
            .with_extension("id", value.id.clone())
            .with_extension("action", value.action));
        }

        Ok(())
    }
}

impl CustomValidator<Option<IntegrationInputGQL>> for IntegrationValidator {
    fn check(
        &self,
        value: &Option<IntegrationInputGQL>,
    ) -> Result<(), async_graphql::InputValueError<Option<IntegrationInputGQL>>> {
        if let Some(vs) = value {
            vs.raise_if_invalid().map_err(|_| {
                async_graphql::InputValueError::custom("Integration is invalid")
                    .with_extension("error", "missing field(s)")
                    .with_extension("kind", vs.kind)
                    .with_extension("action", vs.action)
                    .with_extension("id", vs.id.clone())
            })?;

            // Check if the action is valid
            if !self.limit.contains(&vs.action) {
                return Err(async_graphql::InputValueError::custom(
                    "Action not allowed for this input field",
                )
                .with_extension("id", vs.id.clone())
                .with_extension("action", vs.action));
            }
        }

        Ok(())
    }
}

impl CustomValidator<Option<Vec<IntegrationInputGQL>>> for IntegrationValidator {
    fn check(
        &self,
        value: &Option<Vec<IntegrationInputGQL>>,
    ) -> Result<(), async_graphql::InputValueError<Option<Vec<IntegrationInputGQL>>>> {
        if let Some(vs_opt) = value {
            for (idx, vs) in vs_opt.iter().enumerate() {
                vs.raise_if_invalid().map_err(|_| {
                    async_graphql::InputValueError::custom("One or more integrations are invalid")
                        .with_extension("index", idx)
                        .with_extension("error", "missing field(s)")
                        .with_extension("kind", vs.kind)
                        .with_extension("action", vs.action)
                        .with_extension("id", vs.id.clone())
                })?;

                // Check if the action is valid
                if !self.limit.contains(&vs.action) {
                    return Err(async_graphql::InputValueError::custom(
                        "Action not allowed for this input field",
                    )
                    .with_extension("index", idx)
                    .with_extension("id", vs.id.clone())
                    .with_extension("action", vs.action));
                }
            }
        }

        Ok(())
    }
}

impl From<IntegrationInputGQL> for showtimes_db::m::IntegrationId {
    fn from(value: IntegrationInputGQL) -> Self {
        showtimes_db::m::IntegrationId::new(value.id, value.kind.into())
    }
}

impl From<&IntegrationInputGQL> for showtimes_db::m::IntegrationId {
    fn from(value: &IntegrationInputGQL) -> Self {
        showtimes_db::m::IntegrationId::new(value.id.clone(), value.kind.into())
    }
}

pub struct NonEmptyValidator;

impl CustomValidator<Vec<String>> for NonEmptyValidator {
    fn check(
        &self,
        value: &Vec<String>,
    ) -> Result<(), async_graphql::InputValueError<Vec<String>>> {
        for (idx, vs) in value.iter().enumerate() {
            if vs.trim().is_empty() {
                return Err(
                    async_graphql::InputValueError::custom("Value cannot be empty")
                        .with_extension("index", idx)
                        .with_extension("value", vs.clone()),
                );
            }
        }

        Ok(())
    }
}

impl CustomValidator<Option<Vec<String>>> for NonEmptyValidator {
    fn check(
        &self,
        value: &Option<Vec<String>>,
    ) -> Result<(), async_graphql::InputValueError<Option<Vec<String>>>> {
        if let Some(vs_val) = value {
            for (idx, vs) in vs_val.iter().enumerate() {
                if vs.trim().is_empty() {
                    return Err(
                        async_graphql::InputValueError::custom("Value cannot be empty")
                            .with_extension("index", idx)
                            .with_extension("value", vs.clone()),
                    );
                }
            }
        }

        Ok(())
    }
}

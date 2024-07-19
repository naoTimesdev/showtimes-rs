use bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};
use showtimes_shared::{bson_datetime_opt_serializer, ulid_opt_serializer, ulid_serializer};

use super::{ImageMetadata, IntegrationId, ShowModelHandler};

/// The list of enums holding the project kinds.
#[derive(Debug, Clone, tosho_macros::SerializeEnum, tosho_macros::DeserializeEnum)]
pub enum ProjectKind {
    /// The project is a show.
    Shows,
    /// The project is a literature.
    Literature,
    /// The project is a game.
    Games,
}

tosho_macros::enum_error!(ProjectKindFromStrError);

impl std::str::FromStr for ProjectKind {
    type Err = ProjectKindFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_low = s.to_lowercase();
        match s_low.as_str() {
            "shows" | "show" => Ok(ProjectKind::Shows),
            "literature" | "literatures" | "books" | "book" => Ok(ProjectKind::Literature),
            "games" | "game" => Ok(ProjectKind::Games),
            _ => Err(ProjectKindFromStrError {
                original: s.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for ProjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectKind::Shows => write!(f, "SHOWS"),
            ProjectKind::Literature => write!(f, "LITERATURE"),
            ProjectKind::Games => write!(f, "GAMES"),
        }
    }
}

/// The list of enums holding the project types.
#[derive(Debug, Clone, Default, tosho_macros::SerializeEnum, tosho_macros::DeserializeEnum)]
pub enum ProjectType {
    /// The project is a movie.
    Movies,
    /// The project is a series
    #[default]
    Series,
    /// Oneshots of a series.
    OVAs,
    /// The project is a standard literature books.
    Books,
    /// The project is a manga.
    Manga,
    /// The project is a light novel.
    LightNovel,
    /// The project is a standard games.
    Games,
    /// The project is a visual novel.
    VisualNovel,
}

tosho_macros::enum_error!(ProjectTypeFromStrError);

impl std::str::FromStr for ProjectType {
    type Err = ProjectTypeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_low = s.to_lowercase();
        match s_low.as_str() {
            "movies" | "movie" => Ok(ProjectType::Movies),
            "series" | "shows" | "show" => Ok(ProjectType::Series),
            "ovas" | "ova" => Ok(ProjectType::OVAs),
            "books" | "book" => Ok(ProjectType::Books),
            "manga" => Ok(ProjectType::Manga),
            "light_novel" | "lightnovel" | "ln" => Ok(ProjectType::LightNovel),
            "games" | "game" => Ok(ProjectType::Games),
            "visual_novel" | "vn" => Ok(ProjectType::VisualNovel),
            _ => Err(ProjectTypeFromStrError {
                original: s.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Movies => write!(f, "MOVIES"),
            ProjectType::Series => write!(f, "SERIES"),
            ProjectType::OVAs => write!(f, "OVAs"),
            ProjectType::Books => write!(f, "BOOKS"),
            ProjectType::Manga => write!(f, "MANGA"),
            ProjectType::LightNovel => write!(f, "LIGHT_NOVEL"),
            ProjectType::Games => write!(f, "GAMES"),
            ProjectType::VisualNovel => write!(f, "VISUAL_NOVEL"),
        }
    }
}

impl ProjectType {
    /// Get the kind of the project type.
    pub fn kind(&self) -> ProjectKind {
        match self {
            ProjectType::Movies | ProjectType::Series | ProjectType::OVAs => ProjectKind::Shows,
            ProjectType::Books | ProjectType::Manga | ProjectType::LightNovel => {
                ProjectKind::Literature
            }
            ProjectType::Games | ProjectType::VisualNovel => ProjectKind::Games,
        }
    }
}

fn validate_key(key: &str) -> anyhow::Result<()> {
    if key.is_empty() {
        anyhow::bail!("`key` must not be empty");
    }
    if !key.is_ascii() {
        anyhow::bail!("`key` must be ASCII");
    }
    if !key.contains(' ') && key.to_ascii_uppercase() == key {
        anyhow::bail!("`key` must be uppercase and have no spaces");
    }
    Ok(())
}

fn validate_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("`name` must not be empty");
    }
    Ok(())
}

/// A model to hold each project role in the database.
///
/// Each role is linked to the assignee and status by `key`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    key: String,
    name: String,
}

impl Role {
    /// Create a new role
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> anyhow::Result<Self> {
        let key: String = key.into();
        validate_key(&key)?;
        let name: String = name.into();
        validate_name(&name)?;

        Ok(Role { key, name })
    }

    /// Getter for the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Getter for the name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A model to hold each project role status in the database.
///
/// Each role is linked to each assignee and project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleStatus {
    key: String,
    name: String,
    finished: bool,
}

impl RoleStatus {
    /// Create a new role status
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        finished: bool,
    ) -> anyhow::Result<Self> {
        let key: String = key.into();
        validate_key(&key)?;
        let name: String = name.into();
        validate_name(&name)?;

        Ok(RoleStatus {
            key,
            name,
            finished,
        })
    }

    /// Getter for the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Getter for the name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Getter for the finished status
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Set the finished status
    pub fn set_finished(&mut self, finished: bool) {
        self.finished = finished;
    }

    /// Toggle the finished status
    pub fn toggle_finished(&mut self) {
        self.finished = !self.finished;
    }
}

impl From<Role> for RoleStatus {
    fn from(role: Role) -> Self {
        let r = role.clone();
        RoleStatus {
            key: r.key,
            name: r.name,
            finished: false,
        }
    }
}

/// The model holding each project role assignee in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignee {
    /// The key associated with the assignee.
    key: String,
    /// The assignee itself, if null then it's not assigned.
    #[serde(with = "ulid_opt_serializer")]
    actor: Option<showtimes_shared::ulid::Ulid>,
}

impl RoleAssignee {
    /// Create a new role assignee
    pub fn new(
        key: impl Into<String>,
        actor: Option<showtimes_shared::ulid::Ulid>,
    ) -> anyhow::Result<Self> {
        let key: String = key.into();
        validate_key(&key)?;

        Ok(RoleAssignee { key, actor })
    }

    /// Getter for the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Getter for the actor
    pub fn actor(&self) -> Option<showtimes_shared::ulid::Ulid> {
        self.actor
    }

    /// Set the actor
    pub fn set_actor(&mut self, actor: Option<showtimes_shared::ulid::Ulid>) {
        self.actor = actor;
    }
}

impl From<Role> for RoleAssignee {
    fn from(role: Role) -> Self {
        let r = role.clone();
        RoleAssignee {
            key: r.key,
            actor: None,
        }
    }
}

impl From<&Role> for RoleAssignee {
    fn from(role: &Role) -> Self {
        RoleAssignee {
            key: role.key().to_string(),
            actor: None,
        }
    }
}

/// The model holding the show or books poster
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Poster {
    /// The URL to the poster.
    pub image: ImageMetadata,
    /// The int color code of the poster.
    pub color: Option<u32>,
}

impl Poster {
    /// Create a new show poster with an image.
    pub fn new(image: ImageMetadata) -> Self {
        Poster { image, color: None }
    }

    /// Create a new show poster with an image and color.
    pub fn new_with_color(image: ImageMetadata, color: u32) -> Self {
        Poster {
            image,
            color: Some(color),
        }
    }
}

/// The model holding a status of a single episode/chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeProgress {
    /// The episode/chapter number.
    pub number: u64,
    /// Is the episode/chapter finished/released.
    pub finished: bool,
    /// The airing/release date of the episode/chapter.
    #[serde(with = "bson_datetime_opt_serializer")]
    pub aired: Option<chrono::DateTime<chrono::Utc>>,
    /// The list of roles in the episode/chapter.
    pub statuses: Vec<RoleStatus>,
    /// The delay reason of the episode/chapter.
    pub delay_reason: Option<String>,
}

impl EpisodeProgress {
    pub fn new(number: u64, finished: bool, roles: Vec<Role>) -> Self {
        EpisodeProgress {
            number,
            finished,
            aired: None,
            statuses: roles.into_iter().map(RoleStatus::from).collect(),
            delay_reason: None,
        }
    }
}

/// The model holding project information.
#[derive(Debug, Clone, Serialize, Deserialize, Default, showtimes_derive::ShowModelHandler)]
#[col_name("ShowtimesProjects")]
pub struct Project {
    /// The ID of the project.
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    pub id: showtimes_shared::ulid::Ulid,
    /// The title of the project.
    pub title: String,
    /// The poster of the project.
    pub poster: Poster,
    /// The list of roles in the project.
    pub roles: Vec<Role>,
    /// The list of role assignees in the project.
    pub assignees: Vec<RoleAssignee>,
    /// The list of episode/chapter progress in the project.
    pub progress: Vec<EpisodeProgress>,
    /// The list of aliases of the project.
    pub aliases: Vec<String>,
    /// The integrations of this project.
    ///
    /// Can be used to link to other services like Discord or FansubDB.
    pub integrations: Vec<IntegrationId>,
    /// The server ID creator of the project.
    #[serde(with = "ulid_serializer", default = "ulid_serializer::default")]
    pub creator: showtimes_shared::ulid::Ulid,
    /// The type of the project.
    pub kind: ProjectType,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(
        with = "chrono_datetime_as_bson_datetime",
        default = "chrono::Utc::now"
    )]
    pub updated: chrono::DateTime<chrono::Utc>,
}

impl Project {
    /// Create a new project.
    pub fn new(
        title: impl Into<String>,
        kind: ProjectType,
        creator: showtimes_shared::ulid::Ulid,
    ) -> anyhow::Result<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        let now = chrono::Utc::now();

        Ok(Project {
            id: ulid_serializer::default(),
            title,
            creator,
            kind,
            created: now,
            updated: now,
            ..Default::default()
        })
    }

    /// Create a new project with a poster.
    pub fn new_with_poster(
        title: impl Into<String>,
        kind: ProjectType,
        creator: showtimes_shared::ulid::Ulid,
        poster: Poster,
    ) -> anyhow::Result<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        let now = chrono::Utc::now();

        Ok(Project {
            id: ulid_serializer::default(),
            title,
            poster,
            creator,
            kind,
            created: now,
            updated: now,
            ..Default::default()
        })
    }

    /// Create a new project with a poster and roles.
    pub fn new_with_poster_roles(
        title: impl Into<String>,
        kind: ProjectType,
        creator: showtimes_shared::ulid::Ulid,
        poster: Poster,
        roles: Vec<Role>,
    ) -> anyhow::Result<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        // generate assignee from roles
        let assignees: Vec<RoleAssignee> = roles.iter().map(RoleAssignee::from).collect();
        let now = chrono::Utc::now();

        Ok(Project {
            id: ulid_serializer::default(),
            title,
            poster,
            roles,
            assignees,
            creator,
            kind,
            created: now,
            updated: now,
            ..Default::default()
        })
    }

    /// Create a new project with a poster, roles, and assignees.
    pub fn new_with_poster_roles_assignees(
        title: impl Into<String>,
        kind: ProjectType,
        creator: showtimes_shared::ulid::Ulid,
        poster: Poster,
        roles: Vec<Role>,
        assignees: Vec<RoleAssignee>,
    ) -> anyhow::Result<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        let now = chrono::Utc::now();

        Ok(Project {
            id: ulid_serializer::default(),
            title,
            poster,
            roles,
            assignees,
            creator,
            kind,
            created: now,
            updated: now,
            ..Default::default()
        })
    }

    /// Create a new episode/chapter progress.
    pub fn add_episode(&mut self) {
        let number = self.progress.len() as u64 + 1;
        let roles = self.roles.clone();
        self.progress
            .push(EpisodeProgress::new(number, false, roles));
    }

    /// Create a new episode/chapter progress with specific episode/chapter number.
    pub fn add_episode_with_number(&mut self, number: u64) {
        let roles = self.roles.clone();
        self.progress
            .push(EpisodeProgress::new(number, false, roles));
    }

    /// Remove an episode/chapter progress.
    pub fn remove_episode(&mut self, number: u64) {
        self.progress.retain(|e| e.number != number);
    }

    /// Find an episode/chapter progress by number.
    pub fn find_episode(&self, number: u64) -> Option<&EpisodeProgress> {
        self.progress.iter().find(|e| e.number == number)
    }
}

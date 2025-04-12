use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use showtimes_shared::{ulid_opt_serializer, ulid_serializer};

use crate::errors::{SHDbResult, StringValidationError, StringValidationErrorKind};

use super::{ImageMetadata, IntegrationId, ShowModelHandler};

static DEFAULT_ROLES_SHOWS: LazyLock<Vec<Role>> = LazyLock::new(|| {
    vec![
        Role::new("TL", "Translator").expect("Failed to create role TL"),
        Role::new("TLC", "Translation Checker")
            .expect("Failed to create role TLC")
            .with_order(1),
        Role::new("ENC", "Encoder")
            .expect("Failed to create role ENC")
            .with_order(2),
        Role::new("ED", "Editor")
            .expect("Failed to create role ED")
            .with_order(3),
        Role::new("TM", "Timer")
            .expect("Failed to create role TM")
            .with_order(4),
        Role::new("TS", "Typesetter")
            .expect("Failed to create role TS")
            .with_order(5),
        Role::new("QC", "Quality Checker")
            .expect("Failed to create role QC")
            .with_order(6),
    ]
});
static DEFAULT_ROLES_LITERATURE: LazyLock<Vec<Role>> = LazyLock::new(|| {
    vec![
        Role::new("TL", "Translator").expect("Failed to create role TL"),
        Role::new("TLC", "Translation Checker")
            .expect("Failed to create role TLC")
            .with_order(1),
        Role::new("ED", "Editor")
            .expect("Failed to create role ED")
            .with_order(2),
        Role::new("PR", "Proofreader")
            .expect("Failed to create role PR")
            .with_order(3),
        Role::new("QC", "Quality Checker")
            .expect("Failed to create role QC")
            .with_order(4),
    ]
});
static DEFAULT_ROLES_MANGA: LazyLock<Vec<Role>> = LazyLock::new(|| {
    vec![
        Role::new("TL", "Translator").expect("Failed to create role TL"),
        Role::new("CL", "Cleaner")
            .expect("Failed to create role CL")
            .with_order(1),
        Role::new("RD", "Redrawer")
            .expect("Failed to create role RD")
            .with_order(2),
        Role::new("PR", "Proofreader")
            .expect("Failed to create role PR")
            .with_order(3),
        Role::new("TS", "Typesetter")
            .expect("Failed to create role TS")
            .with_order(4),
        Role::new("QC", "Quality Checker")
            .expect("Failed to create role QC")
            .with_order(5),
    ]
});
static DEFAULT_ROLES_GAMES: LazyLock<Vec<Role>> = LazyLock::new(|| {
    vec![
        Role::new("TL", "Translator").expect("Failed to create role TL"),
        Role::new("TLC", "Translation Checker")
            .expect("Failed to create role TLC")
            .with_order(1),
        Role::new("ED", "Editor")
            .expect("Failed to create role ED")
            .with_order(2),
        Role::new("PRG", "Programming")
            .expect("Failed to create role PRG")
            .with_order(3),
        Role::new("QC", "Quality Checker")
            .expect("Failed to create role QC")
            .with_order(4),
    ]
});
static DEFAULT_ROLES_UNKNOWN: LazyLock<Vec<Role>> =
    LazyLock::new(|| vec![Role::new("TL", "Translator").expect("Failed to create role TL")]);

/// The list of enums holding the project kinds.
#[derive(Debug, Copy, Clone, showtimes_derive::SerdeAutomata)]
#[serde_automata(serialize_rename_all = "SCREAMING_SNAKE_CASE", case_sensitive = false)]
pub enum ProjectKind {
    /// The project is a show.
    #[serde_automata(deser_rename = "show, shows")]
    Shows,
    /// The project is a literature.
    #[serde_automata(deser_rename = "literature, literatures, book, books, novel, novels")]
    Literature,
    /// The project is a manga or comics.
    #[serde_automata(deser_rename = "manga, comic, comics")]
    Manga,
    /// The project is a game.
    #[serde_automata(deser_rename = "game, games")]
    Games,
    /// The project is an unknown kind.
    #[serde_automata(skip)]
    Unknown,
}

impl ProjectKind {
    pub fn default_roles(&self) -> Vec<Role> {
        match self {
            ProjectKind::Shows => DEFAULT_ROLES_SHOWS.clone(),
            ProjectKind::Literature => DEFAULT_ROLES_LITERATURE.clone(),
            ProjectKind::Manga => DEFAULT_ROLES_MANGA.clone(),
            ProjectKind::Games => DEFAULT_ROLES_GAMES.clone(),
            ProjectKind::Unknown => DEFAULT_ROLES_UNKNOWN.clone(),
        }
    }
}

/// The list of enums holding the project types.
#[derive(Debug, Copy, Clone, Default, showtimes_derive::SerdeAutomata)]
#[serde_automata(serialize_rename_all = "SCREAMING_SNAKE_CASE", case_sensitive = false)]
pub enum ProjectType {
    /// The project is a movie.
    #[serde_automata(deser_rename = "movie, movies")]
    Movies,
    /// The project is a series
    #[serde_automata(deser_rename = "show, shows, series")]
    #[default]
    Series,
    /// Oneshots of a series.
    #[serde_automata(ser_rename = "OVAs", deser_rename = "ova, ovas")]
    OVAs,
    /// The project is a standard literature books.
    #[serde_automata(deser_rename = "book, books")]
    Books,
    /// The project is a manga.
    #[serde_automata(deser_rename = "manga, comic, comics")]
    Manga,
    /// The project is a light novel.
    #[serde_automata(deser_rename = "ln, lns, lightnovel, light_novel, lightnovels, light_novels")]
    LightNovel,
    /// The project is a standard games.
    #[serde_automata(deser_rename = "game, games")]
    Games,
    /// The project is a visual novel.
    #[serde_automata(
        deser_rename = "vn, vns, visualnovel, visual_novel, visualnovels, visual_novels"
    )]
    VisualNovel,
    /// The project is an unknown type.
    #[serde_automata(skip)]
    Unknown,
}

impl ProjectType {
    /// Get the kind of the project type.
    pub fn kind(&self) -> ProjectKind {
        match self {
            ProjectType::Movies | ProjectType::Series | ProjectType::OVAs => ProjectKind::Shows,
            ProjectType::Books | ProjectType::LightNovel => ProjectKind::Literature,
            ProjectType::Manga => ProjectKind::Manga,
            ProjectType::Games | ProjectType::VisualNovel => ProjectKind::Games,
            ProjectType::Unknown => ProjectKind::Unknown,
        }
    }

    /// Transfer to a locale string used for translation
    pub fn to_locale(&self) -> &'static str {
        match self {
            ProjectType::Movies => "movie",
            ProjectType::Series => "series",
            ProjectType::OVAs => "ova",
            ProjectType::Books => "book",
            ProjectType::Manga => "manga",
            ProjectType::LightNovel => "light-novel",
            ProjectType::Games => "games",
            ProjectType::VisualNovel => "vn",
            ProjectType::Unknown => "other",
        }
    }
}

impl From<ProjectType> for ProjectKind {
    fn from(t: ProjectType) -> ProjectKind {
        t.kind()
    }
}

impl From<&ProjectType> for ProjectKind {
    fn from(t: &ProjectType) -> ProjectKind {
        t.kind()
    }
}

fn validate_key(key: &str) -> SHDbResult<(), StringValidationError> {
    if key.is_empty() {
        return Err(StringValidationError::new(
            "key",
            StringValidationErrorKind::Empty,
        ));
    }
    if !key.is_ascii() {
        return Err(StringValidationError::new(
            "key",
            StringValidationErrorKind::ASCIIOnly,
        ));
    }
    if !key.contains(' ') && key.to_ascii_uppercase() != key {
        return Err(StringValidationError::new(
            "key",
            StringValidationErrorKind::Contains("uppercase and no spaces".to_string()),
        ));
    }
    Ok(())
}

fn validate_name(name: &str) -> SHDbResult<(), StringValidationError> {
    if name.is_empty() {
        Err(StringValidationError::new(
            "key",
            StringValidationErrorKind::Empty,
        ))
    } else {
        Ok(())
    }
}

/// A model to hold each project role in the database.
///
/// Each role is linked to the assignee and status by `key`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// The order of the role in the project.
    ///
    /// By default it's 0, and it's used to sort the roles in the project.
    order: i32,
    /// The key name that will be used to link the role to the assignee and status.
    key: String,
    /// The name of the role.
    name: String,
}

impl PartialEq for Role {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key() && self.name() == other.name()
    }
}

impl Eq for Role {}

impl PartialOrd for Role {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.order.cmp(&other.order))
    }
}

impl Ord for Role {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

impl Role {
    /// Create a new role
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> SHDbResult<Self> {
        let key: String = key.into();
        validate_key(&key)?;
        let name: String = name.into();
        validate_name(&name)?;

        Ok(Role {
            order: 0,
            key,
            name,
        })
    }

    /// Create a new role with order
    pub fn with_order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    /// Getter for the order
    pub fn order(&self) -> i32 {
        self.order
    }

    /// Getter for the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Getter for the name
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn set_order(&mut self, order: i32) {
        self.order = order;
    }
}

/// A model to hold each project role status in the database.
///
/// Each role is linked to each assignee and project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleStatus {
    key: String,
    finished: bool,
}

impl RoleStatus {
    /// Create a new role status
    pub fn new(key: impl Into<String>, finished: bool) -> SHDbResult<Self> {
        let key: String = key.into();
        validate_key(&key)?;

        Ok(RoleStatus { key, finished })
    }

    /// Getter for the key
    pub fn key(&self) -> &str {
        &self.key
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

impl PartialEq for RoleStatus {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key() && self.finished() == other.finished()
    }
}

impl Eq for RoleStatus {}

impl From<Role> for RoleStatus {
    fn from(role: Role) -> Self {
        RoleStatus {
            key: role.key().to_string(),
            finished: false,
        }
    }
}

impl From<&Role> for RoleStatus {
    fn from(role: &Role) -> Self {
        RoleStatus {
            key: role.key().to_string(),
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

impl PartialEq for RoleAssignee {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key() && self.actor() == other.actor()
    }
}

impl Eq for RoleAssignee {}

impl RoleAssignee {
    /// Create a new role assignee
    pub fn new(
        key: impl Into<String>,
        actor: Option<showtimes_shared::ulid::Ulid>,
    ) -> SHDbResult<Self> {
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
    pub const DEFAULT_COLOR: u32 = 0x1EB5A6;

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
    #[serde(with = "jiff::fmt::serde::timestamp::second::optional")]
    pub aired: Option<jiff::Timestamp>,
    /// The list of roles in the episode/chapter.
    pub statuses: Vec<RoleStatus>,
    /// The delay reason of the episode/chapter.
    pub delay_reason: Option<String>,
}

impl EpisodeProgress {
    pub fn new(number: u64, finished: bool) -> Self {
        EpisodeProgress {
            number,
            finished,
            aired: None,
            statuses: vec![],
            delay_reason: None,
        }
    }

    pub fn new_with_roles(number: u64, finished: bool, roles: &[Role]) -> Self {
        EpisodeProgress {
            number,
            finished,
            aired: None,
            statuses: roles.iter().map(RoleStatus::from).collect(),
            delay_reason: None,
        }
    }

    pub fn with_statuses(&self, statuses: Vec<RoleStatus>) -> Self {
        EpisodeProgress {
            number: self.number,
            finished: self.finished,
            aired: self.aired,
            delay_reason: self.delay_reason.clone(),
            statuses,
        }
    }

    pub fn set_delay_reason(&mut self, reason: impl Into<String>) {
        self.delay_reason = Some(reason.into());
    }

    pub fn clear_delay_reason(&mut self) {
        self.delay_reason = None;
    }

    pub fn set_aired(&mut self, aired: Option<jiff::Timestamp>) {
        self.aired = aired;
    }

    pub fn set_aired_from_unix(&mut self, aired: i64) -> SHDbResult<()> {
        let timestamp = jiff::Timestamp::from_second(aired)
            .map_err(|_| crate::errors::Error::TimeConversionError(aired))?;

        self.aired = Some(timestamp);
        Ok(())
    }

    pub fn set_finished(&mut self, finished: bool) {
        self.finished = finished;
    }

    /// Check if the episode/chapter is progressing.
    pub fn is_progressing(&self) -> bool {
        self.statuses.iter().any(|s| !s.finished)
    }

    /// Check if the episode/chapter is finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Propagate roles changes from the project to the episode/chapter.
    pub fn propagate_roles(&mut self, roles: &[Role]) {
        let roles_keys: Vec<String> = roles.iter().map(|r| r.key.clone()).collect();
        // Update the statuses
        self.statuses.retain(|s| roles_keys.contains(&s.key));
    }

    /// Compare statuses with other project statuses.
    ///
    /// Returns true if there are no changes.
    pub fn compare_statuses(&self, other: &Self) -> bool {
        let mut sort_self = self.statuses.clone();
        let mut sort_other = other.statuses.clone();

        sort_self.sort_by(|a, b| a.key.cmp(&b.key));
        sort_other.sort_by(|a, b| a.key.cmp(&b.key));

        sort_self == sort_other
    }
}

impl PartialEq for EpisodeProgress {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
            && self.finished == other.finished
            && self.aired == other.aired
            && self.compare_statuses(other)
            && self.delay_reason == other.delay_reason
    }
}

impl Eq for EpisodeProgress {}

impl PartialOrd for EpisodeProgress {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.number.cmp(&other.number))
    }
}

impl Ord for EpisodeProgress {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.number.cmp(&other.number)
    }
}

/// The project status information.
///
/// This is only used for marking information for the project.
///
/// If you want to check if the project is finished or not, please check the `progress` field.
///
/// When in archived mode, if this project is synchronized it will not propagate the status.
/// When the other server is syncing project with archived status it will silently update the information
/// without announcing it to the public.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectStatus {
    /// The project is currently ongoing or active.
    ///
    /// User can do any changes in the project.
    #[default]
    Active,
    /// The project is currently paused or in hiatus.
    ///
    /// User can do any changes in the project.
    /// This will give better user experience, when project is stalled.
    Paused,
    /// The current project is dropped or archived.
    ///
    /// Determining archive or dropped status:
    /// - When all episodes/chapters in the project are finished and released, the project is archived.
    /// - Otherwise, the project is dropped.
    ///
    /// User can't do any changes in the project.
    Archived,
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
    /// The status of the project.
    pub status: ProjectStatus,
    /// The type of the project.
    pub kind: ProjectType,
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    pub created: jiff::Timestamp,
    #[serde(
        with = "jiff::fmt::serde::timestamp::second::required",
        default = "jiff::Timestamp::now"
    )]
    pub updated: jiff::Timestamp,
}

impl Project {
    /// Create a new project.
    pub fn new(
        title: impl Into<String>,
        kind: ProjectType,
        creator: showtimes_shared::ulid::Ulid,
    ) -> SHDbResult<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        let now = jiff::Timestamp::now();

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
    ) -> SHDbResult<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        let now = jiff::Timestamp::now();

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
    ) -> SHDbResult<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        // generate assignee from roles
        let assignees: Vec<RoleAssignee> = roles.iter().map(RoleAssignee::from).collect();
        let now = jiff::Timestamp::now();

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
    ) -> SHDbResult<Self> {
        let title: String = title.into();
        validate_name(&title)?;

        let now = jiff::Timestamp::now();

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
        self.progress
            .push(EpisodeProgress::new_with_roles(number, false, &self.roles));
    }

    /// Create a new episode/chapter progress with specific episode/chapter number.
    pub fn add_episode_with_number(&mut self, number: u64) {
        let episode = EpisodeProgress::new_with_roles(number, false, &self.roles);
        self.progress.push(episode);
    }

    /// Create a new episode/chapter progress with specific episode/chapter number and airing date.
    pub fn add_episode_with_number_and_airing(&mut self, number: u64, aired_at: jiff::Timestamp) {
        let mut episode = EpisodeProgress::new_with_roles(number, false, &self.roles);
        episode.set_aired(Some(aired_at));
        self.progress.push(episode);
    }

    /// Remove an episode/chapter progress.
    pub fn remove_episode(&mut self, number: u64) {
        self.progress.retain(|e| e.number != number);
    }

    /// Find an episode/chapter progress by number.
    pub fn find_episode(&self, number: u64) -> Option<&EpisodeProgress> {
        self.progress.iter().find(|e| e.number == number)
    }

    /// Find an episode/chapter progress by number but mutable.
    pub fn find_episode_mut(&mut self, number: u64) -> Option<&mut EpisodeProgress> {
        self.progress.iter_mut().find(|e| e.number == number)
    }

    /// Update an episode/chapter progress.
    pub fn update_episode(&mut self, episode: EpisodeProgress) {
        if let Some(ep) = self
            .progress
            .iter_mut()
            .find(|e| e.number == episode.number)
        {
            *ep = episode;
        }
    }

    /// Add an integration
    pub fn add_integration(&mut self, integration: IntegrationId) {
        self.integrations.push(integration);
    }

    /// Remove an integration
    pub fn remove_integration(&mut self, integration: &IntegrationId) {
        self.integrations.retain(|i| i != integration);
    }

    /// Propagate the roles for assignees.
    pub fn propagate_roles_assignees(&mut self) {
        // Check for roles to be removed
        let roles_keys: Vec<String> = self.roles.iter().map(|r| r.key.clone()).collect();
        // Update the assignees
        self.assignees.retain(|a| roles_keys.contains(&a.key));
        let existing_keys = self
            .assignees
            .iter()
            .map(|a| a.key.clone())
            .collect::<Vec<String>>();

        // Get missing roles
        let missing_roles: Vec<RoleAssignee> = self
            .roles
            .iter()
            .filter(|r| !existing_keys.contains(&r.key))
            .map(RoleAssignee::from)
            .collect();

        if !missing_roles.is_empty() {
            // Add the missing roles
            self.assignees.extend(missing_roles);
        }
    }

    /// Propagate the roles to the assignees and statuses.
    pub fn propagate_roles(&mut self) {
        self.propagate_roles_assignees();
        // Update the statuses
        self.progress.iter_mut().for_each(|e| {
            e.propagate_roles(&self.roles);
        });
    }

    /// Sort ascendingly the progress episodes/chapters by number.
    pub fn sort_progress(&mut self) {
        self.progress.sort();
    }

    /// Create a new Clone or Duplicate of this project with different ID and creator.
    pub fn duplicate(&self, creator: showtimes_shared::ulid::Ulid) -> Self {
        let mut new_project = self.clone();
        let cur_time = jiff::Timestamp::now();
        new_project.id = ulid_serializer::default();
        new_project.creator = creator;
        new_project.created = cur_time;
        new_project.updated = cur_time;
        new_project.unset_id();
        new_project
    }

    /// Compare roles list with other project roles.
    ///
    /// Returns true if the roles are the same.
    pub fn compare_roles(&self, other: &Self) -> bool {
        let mut roles = self.roles.clone();
        let mut other_roles = other.roles.clone();
        roles.sort();
        other_roles.sort();

        // Compare inner roles
        roles == other_roles
    }

    /// Compare assignees list with other project assignees.
    ///
    /// Returns true if the assignees are the same.
    pub fn compare_assignees(&self, other: &Self) -> bool {
        let mut assignees = self.assignees.clone();
        let mut other_assignees = other.assignees.clone();
        assignees.sort_by(|a, b| a.key.cmp(&b.key));
        other_assignees.sort_by(|a, b| a.key.cmp(&b.key));

        // Compare inner assignees
        assignees == other_assignees
    }

    /// Compare progress list with other project progress.
    ///
    /// Returns true if the progress are the same.
    pub fn compare_progress(&self, other: &Self) -> bool {
        let mut progress = self.progress.clone();
        let mut other_progress = other.progress.clone();
        progress.sort();
        other_progress.sort();

        // Compare inner progress
        progress == other_progress
    }
}

use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    Parser, Subcommand,
};

#[derive(Parser)]
#[command(name = "showtimes-migrate")]
#[command(bin_name = "showtimes-migrate")]
#[command(author, version = app_version(), about, long_about = None, styles = cli_styles())]
#[command(propagate_version = true, disable_help_subcommand = true)]
pub(crate) struct MigrationCli {
    #[command(subcommand)]
    pub(crate) command: MigrationCommands,
}

#[derive(Subcommand)]
pub(crate) enum MigrationCommands {
    /// List all available migrations
    #[command(name = "list")]
    List {
        /// Detailed info about the migration
        #[arg(short, long)]
        detailed: bool,
    },
    /// Apply all available migrations
    #[command(name = "up")]
    Up {
        /// Do all the way up
        #[arg(short, long)]
        all: bool,
        /// Specific migration to apply
        #[arg(short, long, default_value = None)]
        name: Option<String>,
    },
    /// Rollback the last migration
    #[command(name = "down")]
    Down {
        /// Do all the way down
        #[arg(short, long)]
        all: bool,
        /// Specific migration to rollback
        #[arg(short, long, default_value = None)]
        name: Option<String>,
    },
    /// Fix up Meilisearch indexes
    #[command(name = "meili-fix")]
    MeiliFix,
}

fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Magenta.on_default() | Effects::BOLD | Effects::UNDERLINE)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::BrightCyan.on_default())
}

fn app_version() -> &'static str {
    let base_ver = env!("CARGO_PKG_VERSION");
    let commit = option_env!("VERSION_WITH_HASH");

    match commit {
        Some(commit) => commit,
        None => base_ver,
    }
}

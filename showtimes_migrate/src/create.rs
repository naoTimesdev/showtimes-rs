use std::path::Path;

use clap::{
    Parser,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};

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

#[derive(Parser)]
#[command(name = "showtimes-migrate-create")]
#[command(bin_name = "showtimes-migrate-create")]
#[command(author, version = app_version(), about, long_about = None, styles = cli_styles())]
#[command(propagate_version = true, disable_help_subcommand = true)]
pub(crate) struct MigrationCli {
    #[arg(short, long, default_value = None)]
    pub(crate) name: Option<String>,
}

const MIGRATION_TEMPLATE: &str = r#"use showtimes_db::{ClientShared, DatabaseShared};

use super::Migration;

pub struct {{name}} {
    client: ClientShared,
    db: DatabaseShared,
}

#[async_trait::async_trait]
impl Migration for {{name}} {
    fn init(client: &ClientShared, db: &DatabaseShared) -> Self {
        Self {
            client: client.clone(),
            db: db.clone(),
        }
    }

    fn name(&self) -> &'static str {
        "{{name}}"
    }

    fn timestamp(&self) -> jiff::Timestamp {
        jiff::civil::datetime({{timestamp_split}}, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    fn clone_box(&self) -> Box<dyn Migration> {
        Box::new(Self {
            client: self.client.clone(),
            db: self.db.clone(),
        })
    }

    async fn up(&self) -> anyhow::Result<()> {
        // TODO: Implement the up migration
        anyhow::bail!("Not implemented")
    }

    async fn down(&self) -> anyhow::Result<()> {
        // TODO: Implement the down migration
        anyhow::bail!("Not implemented")
    }
}
"#;

fn snake_case_to_pascal_case(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;

    for c in name.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[tokio::main]
async fn main() {
    let manifest_path = env!("CARGO_MANIFEST_DIR");
    let migrations_dir = Path::new(manifest_path).join("src").join("migrations");

    if !migrations_dir.exists() {
        eprintln!("Migrations directory does not exist: {:?}", migrations_dir);
        std::process::exit(1);
    }

    let args = MigrationCli::parse();

    let original_name = args
        .name
        .unwrap_or("new_migrations".to_string())
        .replace(" ", "_")
        .replace("-", "_");
    // capitalize the first letter
    let name = original_name
        .chars()
        .next()
        .unwrap()
        .to_uppercase()
        .to_string()
        + &original_name[1..];
    let name = snake_case_to_pascal_case(&name);

    let current_time = jiff::Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC);
    // format YYYYMMDDHHMMSS
    let timestamp = current_time.strftime("%Y%m%d%H%M%S").to_string();

    let migration_file = migrations_dir.join(format!("m{}_{}.rs", timestamp, original_name));

    let timestamp_split = format!(
        "{}, {}, {}, {}, {}, {}",
        current_time.year(),
        current_time.month(),
        current_time.day(),
        current_time.hour(),
        current_time.minute(),
        current_time.second()
    );

    let struct_name = format!("M{}{}", timestamp, name);
    let template = MIGRATION_TEMPLATE
        .replace("{{name}}", &struct_name)
        .replace("{{timestamp_split}}", &timestamp_split);

    std::fs::write(&migration_file, template).unwrap();

    println!("Migration file created: {:?}", &migration_file);
}

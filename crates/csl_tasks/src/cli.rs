use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "csl-tasks")]
#[command(about = "Task management with GitHub Issues synchronization")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, default_value = "tasks")]
    pub task_dir: PathBuf,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "List all tasks")]
    List {
        #[arg(long)]
        status: Option<String>,

        #[arg(long, default_value = "table")]
        format: OutputFormat,

        #[arg(long)]
        with_drift: bool,
    },

    #[command(about = "Get details of a specific task")]
    Get {
        id: u32,

        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },

    #[command(about = "Create a new task")]
    Create {
        #[arg(long)]
        subject: String,

        #[arg(long)]
        description: String,

        #[arg(long)]
        active_form: Option<String>,

        #[arg(long, value_parser = parse_metadata)]
        metadata: Vec<(String, serde_json::Value)>,
    },

    #[command(about = "Update an existing task")]
    Update {
        id: u32,

        #[arg(long)]
        subject: Option<String>,

        #[arg(long)]
        description: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        active_form: Option<String>,

        #[arg(long)]
        add_blocks: Vec<u32>,

        #[arg(long)]
        add_blocked_by: Vec<u32>,

        #[arg(long, value_parser = parse_metadata)]
        metadata: Vec<(String, serde_json::Value)>,
    },

    #[command(about = "Delete a task")]
    Delete { id: u32 },

    #[command(about = "Find the next available task")]
    Next {
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },

    #[command(about = "Claim a task (set status to in_progress)")]
    Claim { id: u32 },

    #[command(about = "Mark a task as completed")]
    Complete { id: u32 },

    #[command(about = "Validate all tasks")]
    Validate,

    #[command(about = "Migrate task IDs to align with GitHub issue numbers")]
    MigrateIds {
        #[arg(long)]
        dry_run: bool,
    },

    #[command(about = "Sync tasks with GitHub Issues")]
    Sync {
        #[arg(long)]
        dry_run: bool,

        #[arg(long, default_value = "to-gh")]
        direction: SyncDirection,

        #[arg(long)]
        github_token: Option<String>,

        #[arg(long)]
        github_repo: Option<String>,
    },

    #[command(about = "Show drift between local tasks and GitHub Issues")]
    SyncStatus {
        #[arg(long)]
        github_token: Option<String>,

        #[arg(long)]
        github_repo: Option<String>,
    },

    #[command(about = "Visualize task dependency graph")]
    Graph {
        #[arg(long, default_value = "ascii")]
        format: GraphFormat,
    },

    #[command(about = "Archive completed tasks")]
    Archive {
        #[arg(long)]
        dry_run: bool,
    },

    #[command(about = "Generate shell completion scripts")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Clone, clap::ValueEnum)]
pub enum GraphFormat {
    Ascii,
    Dot,
}

#[derive(Clone, clap::ValueEnum)]
pub enum SyncDirection {
    ToGh,
    FromGh,
    Both,
}

#[derive(Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Text,
}

fn parse_metadata(s: &str) -> Result<(String, serde_json::Value), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("invalid metadata format: {}", s));
    }

    let key = parts[0].to_string();
    let value = serde_json::from_str(parts[1])
        .or_else(|_| Ok(serde_json::Value::String(parts[1].to_string())))
        .map_err(|e: std::io::Error| e.to_string())?;

    Ok((key, value))
}

use clap::{Parser, Subcommand};
use csln_core::Style;
use schemars::schema_for;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate JSON schema for CSLN styles
    Schema,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Schema => {
            let schema = schema_for!(Style);
            println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        }
    }
}

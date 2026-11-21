//! `world_extract`: builds the world packages (docs/PLAN.md §5.2). Run it through `just`:
//! `just world_extract_9697 <game_dir>`, `just world_extract_9697_data`,
//! `just world_fetch_2627`, `just world_complete_2627`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use openswos_world_extract::commands::{
    assemble_9697, complete_2627, describe_fetched, extract_9697, fetch_2627,
};
use openswos_world_extract::error::ExtractError;

#[derive(Debug, Parser)]
#[command(
    name = "world_extract",
    about = "Builds the world packages (docs/PLAN.md §5.2)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Assemble World 1996-97 from the committed package and your copy of the PC game.
    #[command(name = "assemble-9697")]
    Assemble9697(AssembleArgs),
    /// Extract World 1996-97's committed package (data/worlds/1996-97) from the upstream
    /// checkout and its compiled tables.
    #[command(name = "extract-9697")]
    Extract9697(ExtractArgs),
    /// Download World 2026-27's package and check it against the upstream pin.
    #[command(name = "fetch-2627")]
    Fetch2627,
    /// Complete World 2026-27: its download over World 1996-97, its rules from its baked
    /// install (the oracle).
    #[command(name = "complete-2627")]
    Complete2627(ExtractArgs),
}

#[derive(Debug, Args)]
struct AssembleArgs {
    /// The installed PC game: the folder holding `DATA/`.
    #[arg(long)]
    game: PathBuf,
    /// A new or empty folder to write the package into (default: target/cache/worlds/1996-97,
    /// replaced).
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ExtractArgs {
    /// A new or empty folder to write the package into (default: data/worlds/1996-97,
    /// replaced).
    #[arg(long)]
    out: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("world_extract: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs `cli`'s command, and says what it did.
fn run(cli: Cli) -> Result<String, ExtractError> {
    match cli.command {
        Command::Assemble9697(args) => {
            assemble_9697(&args.game, args.out.as_deref()).map(|written| written.to_string())
        }
        Command::Extract9697(args) => {
            extract_9697(args.out.as_deref()).map(|extracted| extracted.to_string())
        }
        Command::Fetch2627 => fetch_2627().map(|fetched| describe_fetched(&fetched)),
        Command::Complete2627(args) => {
            complete_2627(args.out.as_deref()).map(|completed| completed.to_string())
        }
    }
}

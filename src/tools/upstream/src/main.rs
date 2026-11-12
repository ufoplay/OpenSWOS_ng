//! `upstream`: the migration ledger tool (docs/PLAN.md §8). Run it through `just`:
//! `just upstream_fetch`, `just upstream_sync`, `just upstream_mark ...`,
//! `just upstream_status`, `just upstream_check`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use openswos_upstream::check::check;
use openswos_upstream::commands::{behind_counts, gather_facts, mark_ledger, sync_ledger};
use openswos_upstream::error::{Result, UpstreamError};
use openswos_upstream::fetch::fetch;
use openswos_upstream::ledger::{Area, Ledger, Status};
use openswos_upstream::mark::Changes;
use openswos_upstream::status::summarize;
use openswos_upstream::tracked::upstream_files;
use openswos_upstream::workspace::{CHECKOUT_DIR_ENV, Workspace};

#[derive(Debug, Parser)]
#[command(
    name = "upstream",
    about = "Tracks the migration of upstream OpenSWOS (upstream.toml)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Clone or update the upstream checkout at the pinned commit.
    Fetch,
    /// Add new upstream files as untriaged, and drop removed files holding no ported work.
    Sync,
    /// Update the entries whose paths match the glob patterns.
    Mark(MarkArgs),
    /// Show how far the migration is against the pinned commit.
    Status,
    /// Fail unless the ledger covers exactly the tracked upstream files, consistently.
    Check,
}

#[derive(Debug, Args)]
struct MarkArgs {
    /// Glob patterns over upstream paths (`*` within a directory, `**` across).
    #[arg(required = true)]
    patterns: Vec<String>,
    /// The new area.
    #[arg(long, value_enum)]
    area: Option<Area>,
    /// The new status (partial/done record the pin unless --migrated-at is given).
    #[arg(long, value_enum)]
    status: Option<Status>,
    /// The upstream commit the files were migrated against.
    #[arg(long)]
    migrated_at: Option<String>,
    /// A repository file holding the port (repeatable; replaces the list).
    #[arg(long = "rust")]
    rust: Vec<String>,
    /// A function migrated so far (repeatable; added to the list).
    #[arg(long = "function")]
    functions: Vec<String>,
    /// Notes (an empty string clears them).
    #[arg(long)]
    note: Option<String>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("upstream: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let cwd = std::env::current_dir().map_err(|source| UpstreamError::Read {
        path: PathBuf::from("."),
        source,
    })?;
    let workspace = Workspace::discover(&cwd)?;
    let override_dir = std::env::var_os(CHECKOUT_DIR_ENV).map(PathBuf::from);
    let checkout = workspace.checkout_dir(override_dir.as_deref());
    let ledger_path = workspace.ledger_path();
    match cli.command {
        Command::Fetch => {
            let ledger = Ledger::load(&ledger_path)?;
            fetch(&ledger, &checkout)?;
            println!(
                "upstream at {} checked out in {}",
                ledger.upstream.pinned_commit,
                checkout.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Sync => {
            let report = sync_ledger(&ledger_path, &checkout)?;
            for path in &report.added {
                println!("added (untriaged): {path}");
            }
            for path in &report.dropped {
                println!("dropped (gone upstream): {path}");
            }
            for path in &report.orphaned {
                println!("KEPT, gone upstream but holds ported work: {path}");
            }
            println!(
                "{} added, {} dropped, {} orphaned",
                report.added.len(),
                report.dropped.len(),
                report.orphaned.len()
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Mark(args) => {
            let MarkArgs {
                patterns,
                area,
                status,
                migrated_at,
                rust,
                functions,
                note,
            } = args;
            let changes = Changes {
                area,
                status,
                migrated_at,
                rust,
                functions_done: functions,
                notes: note,
            };
            let changed = mark_ledger(&ledger_path, &patterns, &changes)?;
            println!("{} entries updated", changed.len());
            Ok(ExitCode::SUCCESS)
        }
        Command::Status => {
            let ledger = Ledger::load(&ledger_path)?;
            let behind = behind_counts(&ledger, &checkout)?;
            let upstream = upstream_files(&ledger, &checkout)?;
            let not_in_ledger = upstream
                .iter()
                .filter(|path| !ledger.files.contains_key(*path))
                .count();
            print!("{}", summarize(&ledger, &behind, not_in_ledger));
            Ok(ExitCode::SUCCESS)
        }
        Command::Check => {
            let ledger = Ledger::load(&ledger_path)?;
            let facts = gather_facts(&ledger, &checkout, workspace.root())?;
            let problems = check(&ledger, &facts);
            if problems.is_empty() {
                println!(
                    "upstream.toml covers all {} tracked upstream files at {}",
                    facts.upstream_files.len(),
                    ledger.upstream.pinned_commit
                );
                Ok(ExitCode::SUCCESS)
            } else {
                for problem in &problems {
                    eprintln!("{problem}");
                }
                eprintln!("{} problems in upstream.toml", problems.len());
                Ok(ExitCode::FAILURE)
            }
        }
    }
}

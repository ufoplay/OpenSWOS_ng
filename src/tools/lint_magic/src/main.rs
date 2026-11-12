//! `lint_magic [PATH...]`: lints the Rust files under each path (default: `src`) and
//! fails when any magic number or undocumented constant is found. Run it through
//! `just lint_magic` (part of `just check_code`).

use std::path::PathBuf;
use std::process::ExitCode;

use openswos_lint_magic::scan::lint_paths;

/// The directory linted when no path is given.
const DEFAULT_ROOT: &str = "src";

fn main() -> ExitCode {
    let mut roots: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        roots.push(PathBuf::from(DEFAULT_ROOT));
    }
    match lint_paths(&roots) {
        Ok(findings) if findings.is_empty() => ExitCode::SUCCESS,
        Ok(findings) => {
            for finding in &findings {
                eprintln!("{finding}");
            }
            eprintln!("lint_magic: {} findings", findings.len());
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("lint_magic: {error}");
            ExitCode::FAILURE
        }
    }
}

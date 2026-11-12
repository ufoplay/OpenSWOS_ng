//! The errors of the ledger tool.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::ledger::LEDGER_FILE_NAME;

/// Everything that can go wrong while reading the ledger or talking to git.
#[derive(Debug, Error)]
pub enum UpstreamError {
    /// The `git` program could not be started.
    #[error("cannot run git: {0}")]
    GitSpawn(#[source] io::Error),

    /// A git command exited with an error.
    #[error("`git {args}` failed: {stderr}")]
    Git {
        /// The arguments git was run with.
        args: String,
        /// What git printed on stderr.
        stderr: String,
    },

    /// A git command printed something this tool does not understand.
    #[error("unexpected output from `git {args}`: {output}")]
    GitOutput {
        /// The arguments git was run with.
        args: String,
        /// The offending output.
        output: String,
    },

    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: io::Error,
    },

    /// A file could not be written.
    #[error("cannot write {path}: {source}")]
    Write {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: io::Error,
    },

    /// The ledger is not valid TOML, or does not match its schema.
    #[error("invalid {path}: {source}")]
    Parse {
        /// The ledger file.
        path: PathBuf,
        /// The cause.
        source: Box<toml::de::Error>,
    },

    /// No ledger in the start directory or its parents.
    #[error("no {LEDGER_FILE_NAME} in {start} or its parents")]
    NoLedger {
        /// Where the search started.
        start: PathBuf,
    },

    /// The upstream checkout does not exist, or lacks the pinned commit.
    #[error(
        "the upstream checkout {dir} is missing or lacks the pinned commit: run `just upstream_fetch`"
    )]
    NoCheckout {
        /// The checkout directory.
        dir: PathBuf,
    },

    /// A submodule is checked out at a commit other than the one the ledger records.
    #[error(
        "submodule {path} is at {actual} at the pinned commit, but {LEDGER_FILE_NAME} says {expected}: update the ledger"
    )]
    SubmoduleMismatch {
        /// The submodule path.
        path: String,
        /// The commit the ledger records.
        expected: String,
        /// The commit upstream pins at the pinned commit.
        actual: String,
    },

    /// A glob pattern does not parse.
    #[error("invalid pattern `{pattern}`: {source}")]
    Pattern {
        /// The pattern.
        pattern: String,
        /// The cause.
        source: globset::Error,
    },

    /// A pattern given to `mark` matches no ledger entry.
    #[error("pattern `{pattern}` matches no file of the ledger")]
    NoMatch {
        /// The pattern.
        pattern: String,
    },

    /// A string that should be a full commit hash is not one.
    #[error("`{value}` is not a full commit hash (40 hexadecimal digits)")]
    InvalidCommit {
        /// The offending value.
        value: String,
    },
}

/// The result type of the ledger tool.
pub type Result<T> = std::result::Result<T, UpstreamError>;

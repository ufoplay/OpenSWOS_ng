//! A thin wrapper over the `git` command line.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::error::{Result, UpstreamError};

/// Exit status of `git merge-base --is-ancestor` when the first commit is not an ancestor.
const NOT_ANCESTOR_EXIT_CODE: i32 = 1;

/// A git repository on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    dir: PathBuf,
}

/// What a tree entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A file.
    Blob,
    /// A submodule's pinned commit (a gitlink).
    Commit,
}

/// One entry of a recursive tree listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// What the entry is.
    pub kind: EntryKind,
    /// The object hash: the file's blob, or the submodule's commit.
    pub hash: String,
    /// The path, relative to the repository root.
    pub path: String,
}

impl Git {
    /// The repository at `dir`.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The repository directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Clones `repository` into `dir` without file contents (they are fetched on
    /// checkout) and without checking anything out.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn clone_blobless(repository: &str, dir: &Path) -> Result<Self> {
        let dir_text = dir.to_string_lossy();
        let args = [
            "clone",
            "--filter=blob:none",
            "--no-checkout",
            repository,
            dir_text.as_ref(),
        ];
        let output = Command::new("git")
            .args(args)
            .output()
            .map_err(UpstreamError::GitSpawn)?;
        stdout_of(&args, output)?;
        Ok(Self::new(dir))
    }

    /// Whether `rev` names a commit present in the repository.
    #[must_use]
    pub fn has_commit(&self, rev: &str) -> bool {
        let object = format!("{rev}^{{commit}}");
        self.run(&["cat-file", "-e", object.as_str()]).is_ok()
    }

    /// Fetches the remote's new commits (without file contents).
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn fetch_origin(&self) -> Result<()> {
        self.run(&["fetch", "--filter=blob:none", "origin"])
            .map(drop)
    }

    /// Restricts the working tree to `dirs` (a cone-mode sparse checkout). A submodule's
    /// path may be one of them: `--skip-checks` lets git accept it once the index holds the
    /// submodule as a gitlink rather than a directory.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn sparse_checkout(&self, dirs: &[String]) -> Result<()> {
        let mut args = vec!["sparse-checkout", "set", "--cone", "--skip-checks"];
        args.extend(dirs.iter().map(String::as_str));
        self.run(&args).map(drop)
    }

    /// Checks out `rev` on a detached head.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn checkout_detached(&self, rev: &str) -> Result<()> {
        self.run(&["checkout", "--quiet", "--detach", rev])
            .map(drop)
    }

    /// Initialises and checks out the submodule at `path`, at the commit the current head
    /// pins.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn update_submodule(&self, path: &str) -> Result<()> {
        self.run(&[
            "submodule",
            "update",
            "--init",
            "--filter=blob:none",
            "--",
            path,
        ])
        .map(drop)
    }

    /// The commit the head points at.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn head(&self) -> Result<String> {
        Ok(self.run(&["rev-parse", "HEAD"])?.trim().to_owned())
    }

    /// Every file and gitlink of the tree at `rev`, recursively.
    ///
    /// # Errors
    ///
    /// Fails when git fails or prints an entry this tool does not understand.
    pub fn list_tree(&self, rev: &str) -> Result<Vec<TreeEntry>> {
        let args = ["ls-tree", "-r", "-z", rev];
        let output = self.run(&args)?;
        output
            .split('\0')
            .filter(|record| !record.is_empty())
            .map(|record| parse_tree_entry(record).ok_or_else(|| unexpected(&args, record)))
            .collect()
    }

    /// The commit a submodule at `path` is pinned to in the tree at `rev`, if there is a
    /// submodule there.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn gitlink(&self, rev: &str, path: &str) -> Result<Option<String>> {
        let args = ["ls-tree", "-z", rev, "--", path];
        let output = self.run(&args)?;
        let mut gitlinks = Vec::new();
        for record in output.split('\0').filter(|record| !record.is_empty()) {
            let entry = parse_tree_entry(record).ok_or_else(|| unexpected(&args, record))?;
            match entry.kind {
                EntryKind::Commit => gitlinks.push(entry.hash),
                EntryKind::Blob => {}
            }
        }
        Ok(gitlinks.into_iter().next())
    }

    /// How many commits in `from..to` touch `path`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn commits_touching(&self, from: &str, to: &str, path: &str) -> Result<usize> {
        let range = format!("{from}..{to}");
        let args = ["rev-list", "--count", range.as_str(), "--", path];
        let output = self.run(&args)?;
        output
            .trim()
            .parse()
            .map_err(|_| unexpected(&args, output.trim()))
    }

    /// Whether `ancestor` is `descendant` or one of its ancestors.
    ///
    /// # Errors
    ///
    /// Fails when git fails, e.g. because a commit is unknown.
    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        let args = ["merge-base", "--is-ancestor", ancestor, descendant];
        let output = self.command(&args)?;
        if output.status.success() {
            Ok(true)
        } else if output.status.code() == Some(NOT_ANCESTOR_EXIT_CODE) {
            Ok(false)
        } else {
            stdout_of(&args, output).map(|_| false)
        }
    }

    /// Runs git in the repository and returns its stdout.
    fn run(&self, args: &[&str]) -> Result<String> {
        stdout_of(args, self.command(args)?)
    }

    fn command(&self, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(&self.dir)
            .args(args)
            .output()
            .map_err(UpstreamError::GitSpawn)
    }
}

/// The stdout of a finished git command, or its error.
fn stdout_of(args: &[&str], output: Output) -> Result<String> {
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| UpstreamError::GitOutput {
            args: args.join(" "),
            output: String::from_utf8_lossy(error.as_bytes()).into_owned(),
        })
    } else {
        Err(UpstreamError::Git {
            args: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

fn unexpected(args: &[&str], output: &str) -> UpstreamError {
    UpstreamError::GitOutput {
        args: args.join(" "),
        output: output.to_owned(),
    }
}

/// Parses one `ls-tree` record: `<mode> SP <type> SP <hash> TAB <path>`.
fn parse_tree_entry(record: &str) -> Option<TreeEntry> {
    let (meta, path) = record.split_once('\t')?;
    let mut fields = meta.split(' ');
    let (Some(_mode), Some(kind), Some(hash), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return None;
    };
    let kind = match kind {
        "blob" => EntryKind::Blob,
        "commit" => EntryKind::Commit,
        _other => return None,
    };
    Some(TreeEntry {
        kind,
        hash: hash.to_owned(),
        path: path.to_owned(),
    })
}

#[cfg(test)]
pub(crate) mod test_repo {
    //! Throw-away repositories for the tests.

    use std::path::Path;
    use std::process::Command;

    /// Runs git in `dir` with a neutral identity and no signing; panics on failure.
    pub fn git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("utf-8 output")
            .trim()
            .to_owned()
    }

    /// A new repository in `dir`.
    pub fn init(dir: &Path) {
        std::fs::create_dir_all(dir).expect("repo dir");
        git(dir, &["init", "--quiet"]);
    }

    /// Writes `files` (path, content) and commits them; returns the commit hash.
    pub fn commit(dir: &Path, files: &[(&str, &str)], message: &str) -> String {
        for (path, content) in files {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("parent dir");
            }
            std::fs::write(full, content).expect("file written");
            git(dir, &["add", path]);
        }
        git(dir, &["commit", "--quiet", "--allow-empty", "-m", message]);
        git(dir, &["rev-parse", "HEAD"])
    }
}

#[cfg(test)]
mod tests {
    use super::test_repo::{commit, git, init};
    use super::*;

    #[test]
    fn parses_blob_and_commit_entries() {
        assert_eq!(
            parse_tree_entry("100644 blob abc\tsrc/a.c"),
            Some(TreeEntry {
                kind: EntryKind::Blob,
                hash: "abc".to_owned(),
                path: "src/a.c".to_owned(),
            })
        );
        assert_eq!(
            parse_tree_entry("160000 commit def\tthird_party/x").map(|e| e.kind),
            Some(EntryKind::Commit)
        );
    }

    #[test]
    fn rejects_malformed_entries() {
        assert_eq!(parse_tree_entry("100644 blob abc src/a.c"), None);
        assert_eq!(parse_tree_entry("100644 tree abc\tsrc"), None);
        assert_eq!(parse_tree_entry("100644 blob\tsrc/a.c"), None);
        assert_eq!(parse_tree_entry("100644 blob abc extra\tsrc/a.c"), None);
    }

    #[test]
    fn lists_the_tree_recursively() {
        let dir = tempfile::tempdir().expect("temp dir");
        init(dir.path());
        let head = commit(dir.path(), &[("src/a.c", "a"), ("top.txt", "t")], "first");
        let git = Git::new(dir.path());
        let paths: Vec<String> = git
            .list_tree(&head)
            .expect("listing")
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        assert_eq!(paths, vec!["src/a.c".to_owned(), "top.txt".to_owned()]);
    }

    #[test]
    fn finds_gitlinks() {
        let dir = tempfile::tempdir().expect("temp dir");
        init(dir.path());
        commit(dir.path(), &[("a.txt", "a")], "first");
        let pinned = "87b65278a15007223ec4f387f1027eb4d5cd7625";
        let cacheinfo = format!("160000,{pinned},third_party/engine");
        git(
            dir.path(),
            &["update-index", "--add", "--cacheinfo", cacheinfo.as_str()],
        );
        let head = commit(dir.path(), &[], "add a submodule");
        let repo = Git::new(dir.path());
        assert_eq!(
            repo.gitlink(&head, "third_party/engine").expect("gitlink"),
            Some(pinned.to_owned())
        );
        assert_eq!(repo.gitlink(&head, "a.txt").expect("no gitlink"), None);
        assert_eq!(repo.gitlink(&head, "missing").expect("no entry"), None);
        let kinds: Vec<EntryKind> = repo
            .list_tree(&head)
            .expect("listing")
            .into_iter()
            .map(|entry| entry.kind)
            .collect();
        assert_eq!(kinds, vec![EntryKind::Blob, EntryKind::Commit]);
    }

    #[test]
    fn counts_the_commits_touching_a_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        init(dir.path());
        let first = commit(dir.path(), &[("a.c", "1"), ("b.c", "1")], "first");
        commit(dir.path(), &[("a.c", "2")], "second");
        let third = commit(dir.path(), &[("a.c", "3"), ("b.c", "2")], "third");
        let git = Git::new(dir.path());
        assert_eq!(
            git.commits_touching(&first, &third, "a.c").expect("count"),
            2
        );
        assert_eq!(
            git.commits_touching(&first, &third, "b.c").expect("count"),
            1
        );
        assert_eq!(
            git.commits_touching(&third, &third, "a.c").expect("count"),
            0
        );
    }

    #[test]
    fn tells_ancestors_apart() {
        let dir = tempfile::tempdir().expect("temp dir");
        init(dir.path());
        let first = commit(dir.path(), &[("a.c", "1")], "first");
        let second = commit(dir.path(), &[("a.c", "2")], "second");
        let git = Git::new(dir.path());
        assert!(git.is_ancestor(&first, &second).expect("ancestry"));
        assert!(git.is_ancestor(&second, &second).expect("ancestry"));
        assert!(!git.is_ancestor(&second, &first).expect("ancestry"));
        assert!(
            git.is_ancestor("0000000000000000000000000000000000000000", &second)
                .is_err()
        );
    }

    #[test]
    fn knows_its_commits_and_head() {
        let dir = tempfile::tempdir().expect("temp dir");
        init(dir.path());
        let head = commit(dir.path(), &[("a.c", "1")], "first");
        let git = Git::new(dir.path());
        assert!(git.has_commit(&head));
        assert!(!git.has_commit("0000000000000000000000000000000000000000"));
        assert_eq!(git.head().expect("head"), head);
        assert_eq!(git.dir(), dir.path());
    }

    #[test]
    fn reports_git_failures_with_stderr() {
        let dir = tempfile::tempdir().expect("temp dir");
        let error = Git::new(dir.path()).head().expect_err("not a repository");
        assert!(matches!(error, UpstreamError::Git { .. }), "{error}");
    }
}

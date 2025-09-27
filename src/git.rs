//! Git repository operations and automatic commit management.
//!
//! This module provides functionality for automatically managing git repositories,
//! including creating commits when files change, generating descriptive commit messages,
//! and optionally pushing changes to remote repositories.
//!
//! The main entry point is the `handle_event` function, which processes file change
//! events and creates appropriate git commits with automatically generated messages
//! that describe what files were added, modified, or deleted.
//!
//! ## Key Features
//!
//! - **Automatic commit creation**: Creates commits with staged changes
//! - **Smart commit messages**: Generates descriptive messages showing file changes
//! - **Remote pushing**: Optional automatic pushing to tracked remote branches
//! - **Repository initialization**: Creates git repositories if they don't exist
//! - **SSH authentication**: Supports SSH key authentication for pushing
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use watchers::{git::{handle_event, EventContext}, Config};
//! use std::path::PathBuf;
//!
//! let context = EventContext {
//!     repo_path: PathBuf::from("/path/to/repo"),
//!     config: Config {
//!         watch_dir: PathBuf::from("/path/to/repo"),
//!         commit_delay_secs: 3,
//!         auto_push: true,
//!         config_path: None,
//!     },
//! };
//!
//! // This will create a commit and optionally push if auto_push is enabled
//! handle_event(context);
//! ```

use git2::{BranchType, Cred, Oid, PushOptions, RemoteCallbacks, Repository, Signature, Status, StatusEntry, StatusOptions, Statuses};
use log::{debug, error};
use std::{env, path::{Path, PathBuf}};

use crate::config::Config;

/// Context information passed to git event handlers.
///
/// This struct carries all the necessary information for processing file change events
/// and creating git commits. It's designed to be thread-safe and avoids sharing
/// git repository references across threads by only storing the repository path.
///
/// # Fields
///
/// * `repo_path` - Path to the git repository directory
/// * `config` - Configuration settings including auto-push behavior and timing
///
/// # Example
///
/// ```rust
/// use watchers::{git::EventContext, Config};
/// use std::path::PathBuf;
///
/// let context = EventContext {
///     repo_path: PathBuf::from("/home/user/my-project"),
///     config: Config {
///         watch_dir: PathBuf::from("/home/user/my-project"),
///         commit_delay_secs: 5,
///         auto_push: false,
///         config_path: None,
///     },
/// };
/// ```
#[derive(Clone)]
pub struct EventContext {
    /// Path to the git repository directory
    pub repo_path: PathBuf,
    /// Configuration settings for the watcher
    pub config: Config,
}

/// Opens an existing git repository or creates a new one if it doesn't exist.
///
/// This function first attempts to open a git repository at the specified path.
/// If the repository doesn't exist, it initializes a new git repository at that location.
///
/// # Arguments
///
/// * `repo_path` - Path to the repository directory
///
/// # Returns
///
/// Returns a `Repository` instance on success, or a `git2::Error` if both
/// opening and initialization fail.
///
/// # Example
///
/// ```rust,no_run
/// use watchers::git::open_or_create_repo;
/// use std::path::Path;
///
/// let repo = open_or_create_repo(Path::new("/path/to/repo"))
///     .expect("Failed to open or create repository");
/// ```
pub fn open_or_create_repo(repo_path: &Path) -> Result<Repository, git2::Error> {
    match Repository::open(repo_path) {
        Ok(repo) => Ok(repo),
        Err(_) => Repository::init(repo_path),
    }
}

/// Gets the current status of changed files in the repository.
///
/// This function returns all files that have been modified, added, or deleted
/// in the working directory. It includes untracked files but excludes ignored
/// and unmodified files to focus on changes that should be committed.
///
/// # Arguments
///
/// * `repo` - Reference to the git repository
///
/// # Returns
///
/// Returns a `Statuses` object containing all changed files with their status flags.
/// The lifetime of the returned `Statuses` is tied to the repository reference.
///
/// # Panics
///
/// This function will panic if git2 fails to read the repository status.
/// In future versions, this should be changed to return a `Result`.
///
/// # Example
///
/// ```rust,no_run
/// use watchers::git::{open_or_create_repo, get_changed_files};
/// use std::path::Path;
///
/// let repo = open_or_create_repo(Path::new("/path/to/repo")).unwrap();
/// let changes = get_changed_files(&repo);
///
/// for entry in changes.iter() {
///     println!("Changed file: {:?}", entry.path());
/// }
/// ```
pub fn get_changed_files<'a>(repo: &'a Repository) -> Statuses<'a> {
    // TODO: submodules
    repo.statuses(Some(
        StatusOptions::new()
            .show(git2::StatusShow::Workdir)
            .include_untracked(true) // for newly added files
            .include_ignored(false)
            .include_unmodified(false)
            .include_unreadable(false),
    ))
    .unwrap()
}

/// Handles a file change event by creating a git commit and optionally pushing.
///
/// This is the main entry point for processing file change events. It opens the
/// repository, checks for changes, generates an appropriate commit message, and
/// creates a commit. If `auto_push` is enabled in the configuration, it will
/// also attempt to push the commit to the remote repository.
///
/// # Arguments
///
/// * `context` - Event context containing repository path and configuration
///
/// # Behavior
///
/// 1. Opens or creates the git repository at the specified path
/// 2. Checks for changed files (modified, added, or deleted)
/// 3. If no changes are found, returns early without creating a commit
/// 4. Generates a descriptive commit message based on the changes
/// 5. Creates a commit with all staged changes
/// 6. If `auto_push` is enabled, pushes the commit to the remote repository
///
/// # Error Handling
///
/// Errors are logged to stdout/stderr but do not panic the application.
/// Repository opening failures and push failures are handled gracefully.
///
/// # Example
///
/// ```rust,no_run
/// use watchers::{git::{handle_event, EventContext}, Config};
/// use std::path::PathBuf;
///
/// let context = EventContext {
///     repo_path: PathBuf::from("/path/to/repo"),
///     config: Config {
///         watch_dir: PathBuf::from("/path/to/repo"),
///         commit_delay_secs: 3,
///         auto_push: true,
///         config_path: None,
///     },
/// };
///
/// handle_event(context);
/// ```
pub fn handle_event(context: EventContext) {
    let repo = match open_or_create_repo(&context.repo_path) {
        Ok(repo) => repo,
        Err(e) => {
            println!("Failed to open repository: {}", e);
            return;
        }
    };

    let changed_files = get_changed_files(&repo);
    if changed_files.is_empty() {
        return;
    }

    let message = get_commit_message(&changed_files);
    create_commit(&repo, Some(&message)).unwrap();
    debug!("creating commit");
    if context.config.auto_push {
        debug!("pushing commit");
        match push_commits(&repo) {
            Ok(_) => (),
            Err(e) => println!("Failed to push with error: {}", e),
        };
        debug!("pushed commit");
    }
}

/// Creates a git commit with all current changes.
///
/// This function stages all changes in the working directory and creates a commit.
/// It handles both initial commits (when no HEAD exists) and regular commits with
/// parent commits. The commit author and committer are determined from the git
/// configuration, falling back to "Watchers" if not configured.
///
/// # Arguments
///
/// * `repo` - Reference to the git repository
/// * `message` - Optional commit message (defaults to "Autocommit")
///
/// # Returns
///
/// Returns the `Oid` (object ID) of the created commit on success, or a `git2::Error`
/// if the commit creation fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The repository index cannot be accessed
/// - Files cannot be staged
/// - The tree cannot be written
/// - Git configuration cannot be read
/// - The commit cannot be created
///
/// # Example
///
/// ```rust,no_run
/// use watchers::git::{open_or_create_repo, create_commit};
/// use std::path::Path;
///
/// let repo = open_or_create_repo(Path::new("/path/to/repo")).unwrap();
/// let commit_id = create_commit(&repo, Some("My commit message"))
///     .expect("Failed to create commit");
/// println!("Created commit: {}", commit_id);
/// ```
pub fn create_commit(repo: &git2::Repository, message: Option<&str>) -> Result<Oid, git2::Error> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let config = repo.config()?;
    let signature = Signature::now(
        config.get_entry("user.name")?.value().unwrap_or("Watchers"),
        config
            .get_entry("user.email")?
            .value()
            .unwrap_or("Watchers"),
    )?;

    let message = message.unwrap_or("Autocommit");

    match repo.head() {
        Ok(head) => {
            let parent_commit = head.peel_to_commit()?;
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&parent_commit],
            )
        }
        Err(_) => {
            // Initial commit - no parents
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[],
            )
        }
    }
}

/// Generates a descriptive commit message based on file changes.
///
/// This function analyzes the git status and creates a two-part commit message:
/// 1. A summary line showing counts of deleted, modified, and added files
/// 2. A detailed section listing the specific files that changed
///
/// The message format follows conventional git practices with a summary line
/// followed by a detailed description separated by a blank line.
///
/// # Arguments
///
/// * `changed_files` - Git status information containing file changes
///
/// # Returns
///
/// Returns a formatted commit message string with summary and details.
///
/// # Example Output
///
/// ```text
/// Modified 2, Added 1
///
/// Modified:
///   src/main.rs
///   README.md
/// Added:
///   src/config.rs
/// ```
///
/// # Example
///
/// ```rust,no_run
/// use watchers::git::{open_or_create_repo, get_changed_files};
/// use std::path::Path;
///
/// let repo = open_or_create_repo(Path::new("/path/to/repo")).unwrap();
/// let changes = get_changed_files(&repo);
/// // Note: get_commit_message is private, this is just for documentation
/// ```
fn get_commit_message(changed_files: &Statuses) -> String {
    // should these be comparing to index instead of working tree?
    // commit hasn't happened yet
    let deleted: Vec<StatusEntry> = changed_files
        .iter()
        .filter(|f| f.status().contains(Status::WT_DELETED))
        .collect();
    let modified: Vec<StatusEntry> = changed_files
        .iter()
        .filter(|f| f.status().contains(Status::WT_MODIFIED))
        .collect();
    let new: Vec<StatusEntry> = changed_files
        .iter()
        .filter(|f| f.status().contains(Status::WT_NEW))
        .collect();

    // NOTE: keep the order of these two arrays synced
    let actions = ["Deleted", "Modified", "Added"];
    let types = [deleted, modified, new];

    let summary = types
        .iter()
        .enumerate()
        .filter_map(|(i, ls)| {
            if !ls.is_empty() {
                Some(format!("{} {}", actions[i], ls.len()))
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join(", ");

    let desc = types
        .iter()
        .enumerate()
        .filter_map(|(i, ls)| {
            let mut lines = vec![format!("{}:", actions[i])];
            for file in ls {
                lines.push(format!("  {}", file.path().unwrap_or("Unknown file"),));
            }

            if lines.len() > 1 {
                Some(lines.join("\n"))
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    [summary, desc].join("\n\n")
}

/// Pushes commits to the remote repository.
///
/// This function automatically determines the appropriate remote and branch to push to
/// by examining the current branch's upstream configuration. If no upstream is configured,
/// it defaults to pushing to "origin" with the same branch name.
///
/// # Arguments
///
/// * `repo` - Reference to the git repository
///
/// # Returns
///
/// Returns `Ok(())` on successful push, or a `git2::Error` if the push fails.
///
/// # Authentication
///
/// Currently uses SSH key authentication with a hardcoded path to `~/.ssh/id_ed25519`.
/// Future versions should support configurable authentication methods.
///
/// # Errors
///
/// This function will return an error if:
/// - The repository HEAD cannot be read
/// - The current branch cannot be found
/// - The remote repository cannot be accessed
/// - Authentication fails
/// - The push operation fails
///
/// # Example
///
/// ```rust,no_run
/// use watchers::git::open_or_create_repo;
/// use std::path::Path;
///
/// let repo = open_or_create_repo(Path::new("/path/to/repo")).unwrap();
/// // Note: push_commits is private, this is just for documentation
/// ```
fn push_commits(repo: &Repository) -> Result<(), git2::Error> {
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("main");
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let (remote_name, remote_branch) = if let Ok(upstream) = branch.upstream() {
        let upstream_name = upstream.name()?.unwrap_or("origin/main");
        let parts: Vec<&str> = upstream_name.splitn(2, '/').collect();
        (
            parts[0].to_string(),
            parts.get(1).unwrap_or(&branch_name).to_string(),
        )
    } else {
        ("origin".to_string(), branch_name.to_string())
    };

    let refspec = format!("refs/heads/{}:refs/heads/{}", remote_branch, remote_branch);

    let mut remote = repo.find_remote(&remote_name)?;
    let mut push_options = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_ed25519", env::var("HOME").unwrap())),
            None,
        )
    });

    callbacks.push_update_reference(|ref_name, status| {
        if let Some(status) = status {
            error!("Failed to push ref: {}. Status: {}", ref_name, status);
        }
        Ok(())
    });

    push_options.remote_callbacks(callbacks);

    remote.push(&[&refspec], Some(&mut push_options))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_open_or_create_repo_same_repo() {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Create repo and make initial commit
        let repo1 = open_or_create_repo(&repo_path).unwrap();

        // Configure user for commits
        let mut config = repo1.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create a test file and commit
        fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
        let commit_id = create_commit(&repo1, Some("Test commit")).unwrap();

        // Open the "same" repo again
        let repo2 = open_or_create_repo(&repo_path).unwrap();

        // Both should see the same commit
        let commit1 = repo1.find_commit(commit_id).unwrap();
        let commit2 = repo2.find_commit(commit_id).unwrap();

        assert_eq!(commit1.id(), commit2.id());
        assert_eq!(commit1.message(), commit2.message());
        assert_eq!(repo1.path(), repo2.path());
    }

    #[test]
    fn test_create_commit() {
        let temp_dir = tempdir().unwrap();
        let repo = Repository::init(&temp_dir).unwrap();

        // Configure user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create a test file
        fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let result = create_commit(&repo, Some("Test commit"));
        assert!(result.is_ok());

        // Verify commit was created
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Test commit");
    }

    #[test]
    fn test_get_commit_message_formatting() {
        let temp_dir = tempdir().unwrap();
        let repo = Repository::init(&temp_dir).unwrap();

        // Configure user
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create and commit initial files
        fs::write(temp_dir.path().join("modified.txt"), "original content").unwrap();
        create_commit(&repo, Some("Initial commit")).unwrap();

        // Modify existing file and add new files
        fs::write(temp_dir.path().join("modified.txt"), "updated content").unwrap();
        fs::write(temp_dir.path().join("new1.txt"), "new file 1").unwrap();
        fs::write(temp_dir.path().join("new2.txt"), "new file 2").unwrap();

        let statuses = get_changed_files(&repo);
        let message = get_commit_message(&statuses);

        assert!(message.contains("Modified 1"));
        assert!(message.contains("Added 2"));
        assert!(message.contains("modified.txt"));
        assert!(message.contains("new1.txt"));
        assert!(message.contains("new2.txt"));
    }

    #[test]
    fn test_get_changed_files() {
        let temp_dir = tempdir().unwrap();
        let repo = Repository::init(&temp_dir).unwrap();

        // No changes initially (empty repo)
        let statuses = get_changed_files(&repo);
        assert!(statuses.is_empty());

        // Create a file
        fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

        let statuses = get_changed_files(&repo);
        assert!(!statuses.is_empty());

        // Should detect the new file
        let entries: Vec<_> = statuses.iter().collect();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].status().contains(Status::WT_NEW));
        assert_eq!(entries[0].path().unwrap(), "test.txt");
    }
}

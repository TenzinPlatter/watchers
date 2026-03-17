use anyhow::{Context, Result};
use log::{debug, error};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::config::Config;

#[derive(Clone)]
pub struct EventContext {
    pub repo_path: PathBuf,
    pub config: Config,
}

fn git(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy()])
        .args(args)
        .output()
        .with_context(|| format!("Failed to run git {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args[0], stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn open_or_create_repo(repo_path: &Path) -> Result<()> {
    // Check if we're in a git repo
    if git(repo_path, &["rev-parse", "--git-dir"]).is_err() {
        git(repo_path, &["init"])?;
    }
    Ok(())
}

struct ChangedFiles {
    deleted: Vec<String>,
    modified: Vec<String>,
    added: Vec<String>,
}

impl ChangedFiles {
    fn is_empty(&self) -> bool {
        self.deleted.is_empty() && self.modified.is_empty() && self.added.is_empty()
    }
}

fn get_changed_files(repo_path: &Path) -> Result<ChangedFiles> {
    // --porcelain gives stable, parseable output
    let output = git(repo_path, &["status", "--porcelain"])?;

    let mut deleted = Vec::new();
    let mut modified = Vec::new();
    let mut added = Vec::new();

    for line in output.lines() {
        if line.len() < 4 {
            continue;
        }
        // Porcelain format: XY filename
        // We care about the working tree status (second char) for unstaged changes
        let xy = &line[..2];
        let file = line[3..].to_string();

        match xy {
            // Deleted in worktree
            " D" => deleted.push(file),
            // Untracked (new) file
            "??" => added.push(file),
            // Modified in worktree, or any other status indicating a change
            _ => modified.push(file),
        }
    }

    Ok(ChangedFiles {
        deleted,
        modified,
        added,
    })
}

pub fn handle_event(context: EventContext) {
    if let Err(e) = open_or_create_repo(&context.repo_path) {
        error!("Failed to open repository: {}", e);
        return;
    }

    let changed_files = match get_changed_files(&context.repo_path) {
        Ok(files) => files,
        Err(e) => {
            error!("Failed to get changed files: {}", e);
            return;
        }
    };

    if changed_files.is_empty() {
        debug!("No changed files");
        return;
    }

    if let Err(e) = commit_submodule_changes(&context) {
        error!("Failed to commit submodule changes: {}", e);
    }

    // Re-check after submodule commits may have changed status
    let changed_files = match get_changed_files(&context.repo_path) {
        Ok(files) => files,
        Err(e) => {
            error!("Failed to get changed files: {}", e);
            return;
        }
    };

    if changed_files.is_empty() {
        debug!("No changed files after submodule commits");
        return;
    }

    let message = get_commit_message(&changed_files);
    if let Err(e) = create_commit(&context.repo_path, &message) {
        error!("Failed to create commit: {}", e);
        return;
    }
    debug!("creating commit");

    if context.config.auto_push {
        debug!("pushing commit");
        match push_commits(&context.repo_path) {
            Ok(_) => (),
            Err(e) => error!("Failed to push with error: {}", e),
        };
        debug!("pushed commit");
    }
}

fn create_commit(repo_path: &Path, message: &str) -> Result<()> {
    // Stage all changes
    git(repo_path, &["add", "-A"])?;
    git(repo_path, &["commit", "-m", message])?;
    Ok(())
}

fn get_commit_message(changed_files: &ChangedFiles) -> String {
    let actions = ["Deleted", "Modified", "Added"];
    let types = [
        &changed_files.deleted,
        &changed_files.modified,
        &changed_files.added,
    ];

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
            if ls.is_empty() {
                return None;
            }
            let mut lines = vec![format!("{}:", actions[i])];
            for file in *ls {
                lines.push(format!("  {}", file));
            }
            Some(lines.join("\n"))
        })
        .collect::<Vec<String>>()
        .join("\n");

    [summary, desc].join("\n\n")
}

fn pull_and_rebase(repo_path: &Path) -> Result<bool> {
    // Fetch from remote
    if let Err(e) = git(repo_path, &["fetch"]) {
        debug!("Fetch failed: {}, skipping rebase", e);
        return Ok(true);
    }

    // Check if there's an upstream branch
    let upstream = match git(repo_path, &["rev-parse", "--abbrev-ref", "@{upstream}"]) {
        Ok(u) => u,
        Err(_) => return Ok(true), // No upstream tracking branch
    };

    debug!("Upstream: {}", upstream);

    // Attempt rebase, abort on conflicts
    match git(repo_path, &["rebase", &upstream]) {
        Ok(_) => {
            debug!("Rebase completed successfully");
            Ok(true)
        }
        Err(e) => {
            debug!("Rebase failed: {}, aborting", e);
            let _ = git(repo_path, &["rebase", "--abort"]);
            Ok(false)
        }
    }
}

fn push_commits(repo_path: &Path) -> Result<()> {
    match pull_and_rebase(repo_path) {
        Ok(true) => debug!("Pull rebase succeeded or not needed"),
        Ok(false) => debug!("Skipping rebase due to conflicts, will attempt push anyway"),
        Err(e) => debug!("Pull rebase failed: {}, will attempt push anyway", e),
    }

    git(repo_path, &["push"])?;
    Ok(())
}

fn commit_submodule_changes(context: &EventContext) -> Result<()> {
    let output = git(
        &context.repo_path,
        &["submodule", "foreach", "--quiet", "echo $sm_path"],
    )?;

    if output.is_empty() {
        return Ok(());
    }

    for submodule_rel_path in output.lines() {
        let submodule_path = context.repo_path.join(submodule_rel_path);

        let changed_files = match get_changed_files(&submodule_path) {
            Ok(files) => files,
            Err(e) => {
                error!(
                    "Failed to get changed files for submodule at {:?}: {}",
                    submodule_path, e
                );
                continue;
            }
        };

        if changed_files.is_empty() {
            continue;
        }

        let message = get_commit_message(&changed_files);
        if let Err(e) = create_commit(&submodule_path, &message) {
            error!(
                "Failed to commit submodule changes at {:?}: {}",
                submodule_path, e
            );
            continue;
        }

        debug!("Created commit in submodule: {:?}", submodule_path);

        if context.config.auto_push {
            if let Err(e) = push_commits(&submodule_path) {
                error!("Failed to push submodule at {:?}: {}", submodule_path, e);
            } else {
                debug!("Pushed submodule: {:?}", submodule_path);
            }
        }
    }

    Ok(())
}

pub fn is_git_ignored<P: AsRef<Path>>(paths: &[P]) -> Result<bool> {
    if paths.is_empty() {
        return Ok(false);
    }

    // Find the repo root from the first path
    let first_parent = paths[0]
        .as_ref()
        .parent()
        .context("Path has no parent")?;

    for p in paths {
        let output = Command::new("git")
            .args(["-C", &first_parent.to_string_lossy()])
            .args(["check-ignore", "-q"])
            .arg(p.as_ref())
            .output()
            .context("Failed to run git check-ignore")?;

        if output.status.success() {
            return Ok(true);
        }
    }

    Ok(false)
}

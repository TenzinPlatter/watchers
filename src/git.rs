use anyhow::Result;
use git2::{
    BranchType, Cred, Oid, PushOptions, RemoteCallbacks, Repository, Signature, Status,
    StatusEntry, StatusOptions, Statuses,
};
use log::{debug, error};
use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
};

use crate::config::Config;

#[derive(Clone)]
pub struct EventContext {
    pub repo_path: PathBuf,
    pub config: Config,
}

pub fn open_or_create_repo(repo_path: &Path) -> Result<Repository, git2::Error> {
    match Repository::discover(repo_path) {
        Ok(repo) => Ok(repo),
        Err(_) => Repository::init(repo_path),
    }
}

pub fn get_changed_files<'a>(repo: &'a Repository) -> Result<Statuses<'a>, git2::Error> {
    repo.statuses(Some(
        StatusOptions::new()
            .show(git2::StatusShow::Workdir)
            .include_untracked(true) // for newly added files
            .include_ignored(false)
            .include_unmodified(false)
            .include_unreadable(false),
    ))
}

pub fn handle_event(context: EventContext) {
    let repo = match open_or_create_repo(&context.repo_path) {
        Ok(repo) => repo,
        Err(e) => {
            println!("Failed to open repository: {}", e);
            return;
        }
    };

    let changed_files = match get_changed_files(&repo) {
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

    if let Err(e) = commit_submodule_changes(&repo, &context) {
        error!("Failed to commit submodule changes: {}", e);
    }

    let message = get_commit_message(&changed_files);
    if let Err(e) = create_commit(&repo, &changed_files, Some(&message)) {
        error!("Failed to create commit: {}", e);
        return;
    }
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

pub fn create_commit(
    repo: &git2::Repository,
    changed_files: &Statuses,
    message: Option<&str>,
) -> Result<Oid, git2::Error> {
    let mut index = repo.index()?;

    // Build submodule lookup for efficient checking
    let submodule_paths: HashSet<PathBuf> = repo
        .submodules()?
        .iter()
        .map(|s| PathBuf::from(s.path()))
        .collect();

    // Stage files individually with submodule-aware handling
    for entry in changed_files.iter() {
        let Some(path_str) = entry.path() else {
            continue;
        };

        let path = Path::new(path_str);

        // Handle regular files
        if !submodule_paths.contains(path) {
            let status = entry.status();

            // Use remove_path for deleted files, add_path for everything else
            if status.contains(Status::WT_DELETED) {
                if let Err(e) = index.remove_path(path) {
                    error!("Failed to remove {}: {}", path_str, e);
                }

                continue;
            }

            if let Err(e) = index.add_path(path) {
                error!("Failed to add {}: {}", path_str, e);
            }

            continue;
        }

        // Handle submodules
        match repo.find_submodule(path_str) {
            Ok(mut submodule) => {
                if let Err(e) = submodule.add_to_index(true) {
                    error!("Failed to stage submodule {}: {}", path_str, e);
                }
            }
            Err(e) => error!("Failed to find submodule {}: {}", path_str, e),
        }
    }

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
            repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
        }
    }
}

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

    // TODO: handle more auth methods
    callbacks.credentials(|url, username_from_url, allowed_types| {
        use git2::CredentialType;

        // Try SSH key first if allowed
        if allowed_types.contains(CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            let home = env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            return Cred::ssh_key(
                username,
                None,
                std::path::Path::new(&format!("{}/.ssh/id_ed25519", home)),
                None,
            );
        }

        // Try credential helper for HTTPS
        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            return Cred::credential_helper(&git2::Config::open_default()?, url, username_from_url);
        }

        // Fallback to default credentials
        Cred::default()
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

fn commit_submodule_changes(repo: &Repository, context: &EventContext) -> Result<()> {
    for submodule in repo.submodules()? {
        let submodule_path = context.repo_path.join(submodule.path());

        // Open the submodule repository
        let sub_repo = match Repository::discover(&submodule_path) {
            Ok(repo) => repo,
            Err(e) => {
                error!("Failed to open submodule at {:?}: {}", submodule_path, e);
                continue;
            }
        };

        // Check if there are changes
        let changed_files = match get_changed_files(&sub_repo) {
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

        // Create commit with message
        let message = get_commit_message(&changed_files);
        if let Err(e) = create_commit(&sub_repo, &changed_files, Some(&message)) {
            error!(
                "Failed to commit submodule changes at {:?}: {}",
                submodule_path, e
            );
            continue;
        }

        debug!("Created commit in submodule: {:?}", submodule_path);

        // Push if auto_push is enabled
        if context.config.auto_push {
            if let Err(e) = push_commits(&sub_repo) {
                error!("Failed to push submodule at {:?}: {}", submodule_path, e);
            } else {
                debug!("Pushed submodule: {:?}", submodule_path);
            }
        }
    }

    Ok(())
}

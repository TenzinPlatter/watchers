use git2::{BranchType, Cred, Oid, PushOptions, RemoteCallbacks, Repository, Signature, Status, StatusEntry, StatusOptions, Statuses};
use log::{debug, error};
use std::{env, path::{Path, PathBuf}};

use crate::config::Config;

#[derive(Clone)]
pub struct EventContext {
    pub repo_path: PathBuf,
    pub config: Config,
}

pub fn open_or_create_repo(repo_path: &Path) -> Result<Repository, git2::Error> {
    match Repository::open(repo_path) {
        Ok(repo) => Ok(repo),
        Err(_) => Repository::init(repo_path),
    }
}

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

    let parent_commit = repo.head()?.peel_to_commit()?;
    let message = message.unwrap_or("Autocommit");

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )
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

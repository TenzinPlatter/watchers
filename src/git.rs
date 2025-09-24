use git2::{Repository, StatusOptions, Statuses};
use notify::Event;
use std::path::Path;

use crate::file_utils::was_modification;

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

pub fn handle_event(repo: &git2::Repository, ev: &Event) {
    if !was_modification(ev) {
        return;
    }

    let changed_files = get_changed_files(repo);
    for file in &changed_files {
        println!("file changed: {:?}", file.path());
    }
}


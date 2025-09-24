pub mod config;
pub mod file_utils;
pub mod git;

use notify::{Event, RecursiveMode, Watcher};
use std::{path::Path, sync::mpsc};
use crate::git::{handle_event, open_or_create_repo};

pub fn watch_repo(watch_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new(watch_dir), RecursiveMode::Recursive)?;

    let repo = open_or_create_repo(watch_dir)?;

    loop {
        match rx.recv() {
            Err(e) => println!("watch error: {:?}", e),
            Ok(ev) => {
                if let Ok(ev) = ev {
                    handle_event(&repo, &ev);
                }
            }
        }
    }
}

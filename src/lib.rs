pub mod file_utils;
pub mod config;

use std::{path::Path, sync::mpsc};
use notify::{Event, RecursiveMode, Result, Watcher};

use crate::file_utils::was_modification;

pub fn watch_directory(watch_dir: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new(watch_dir), RecursiveMode::Recursive)?;
    // Block forever, printing out events as they come in
    for res in rx {
        match res {
            Err(e) => println!("watch error: {:?}", e),
            Ok(e) => {
                if was_modification(e) {
                    println!("Something in subdir was modified");
                }
            },
        }
    }

    Ok(())
}

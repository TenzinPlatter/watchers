pub mod config;
pub mod debouncer;
pub mod file_utils;
pub mod git;

use crate::{
    config::Config, debouncer::Debouncer, file_utils::was_modification, git::EventContext,
};
use log::debug;
use notify::{Event, RecursiveMode};
use std::{sync::mpsc, time::Duration};

pub struct Watcher<'a, F> {
    pub config: &'a Config,
    pub debouncer: Debouncer<F>,
}

impl<'a, F> Watcher<'a, F>
where
    F: FnMut(EventContext) + Send + 'static,
{
    pub fn new(config: &'a Config, debouncer_cb: F) -> Self {
        let debouncer = Debouncer::new(debouncer_cb, Duration::from_secs(config.commit_delay_secs as u64));
        Self { config, debouncer }
    }

    pub fn trigger_debouncer(&mut self) {
        debug!("triggering debouncer");
        let context = EventContext {
            repo_path: self.config.watch_dir.clone(),
            config: self.config.clone(),
        };
        self.debouncer.on_event(context);
    }
}

pub fn watch_repo<F>(watcher: &mut Watcher<F>) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(EventContext) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut fs_watcher = notify::recommended_watcher(tx)?;
    notify::Watcher::watch(
        &mut fs_watcher,
        &watcher.config.watch_dir,
        RecursiveMode::Recursive,
    )?;

    loop {
        match rx.recv() {
            Err(e) => println!("watch error: {:?}", e),
            Ok(ev) => {
                if let Ok(ev) = ev
                    && was_modification(&ev)
                        // TODO: && not_ignored(&ev)
                {
                    debug!("got modification: {:?}", ev);
                    watcher.trigger_debouncer();
                }
            }
        }
    }
}

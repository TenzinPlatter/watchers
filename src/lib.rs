//! # Watchers
//!
//! A file system watcher that automatically creates git commits when files change.
//!
//! ## Overview
//!
//! This library provides a debounced file watcher that monitors a directory for changes
//! and automatically creates git commits after a configurable quiet period. It's designed
//! to help with automatic versioning of documents, configuration files, or any other
//! files that benefit from granular git history.
//!
//! ## Architecture
//!
//! The system consists of several key components:
//!
//! - **Watcher**: Main orchestrator that sets up file system watching
//! - **Debouncer**: Thread-safe timer that delays commit creation until file activity stops
//! - **EventContext**: Helper struct that carries repository and configuration data to callbacks
//! - **Git operations**: Functions for creating commits and managing repository state
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use watchers::{Watcher, Config, git::handle_event, watch_repo};
//! use std::time::Duration;
//!
//! let config = Config {
//!     watch_dir: "/path/to/watch".into(),
//!     commit_delay_secs: 3,
//!     auto_push: true,
//!     config_path: None,
//! };
//!
//! let mut watcher = Watcher::new(&config, |context| {
//!     handle_event(context);
//! });
//!
//! // This will block and watch for file changes
//! watch_repo(&mut watcher).unwrap();
//! ```
//!
//! ## Features
//!
//! - **Debounced commits**: Only creates commits after file activity stops
//! - **Automatic push**: Optionally push commits to remote repository
//! - **Configurable delays**: Customize how long to wait before creating commits
//! - **Thread-safe**: Uses condition variables for efficient timer management
//! - **Git integration**: Full git2 integration for repository operations

pub mod config;
pub mod debouncer;
pub mod file_utils;
pub mod git;

pub use crate::config::Config;

use crate::{
    debouncer::Debouncer, file_utils::was_modification, git::EventContext,
};
use log::debug;
use notify::{Event, RecursiveMode};
use std::{sync::mpsc, time::Duration};

/// Main file watcher that orchestrates debounced git commits.
///
/// The `Watcher` combines file system monitoring with a debouncing mechanism
/// to automatically create git commits when files change, but only after
/// a configurable quiet period to avoid excessive commits during rapid changes.
///
/// # Type Parameters
///
/// * `F` - A callback function that takes an `EventContext` and handles the commit logic
///
/// # Example
///
/// ```rust,no_run
/// use watchers::{Watcher, Config, git::handle_event};
/// use std::path::PathBuf;
///
/// let config = Config {
///     watch_dir: PathBuf::from("/path/to/watch"),
///     commit_delay_secs: 3,
///     auto_push: false,
///     config_path: None,
/// };
///
/// let watcher = Watcher::new(&config, |context| {
///     handle_event(context);
/// });
/// ```
pub struct Watcher<'a, F> {
    /// Reference to the configuration settings
    pub config: &'a Config,
    /// The debouncer that delays callback execution until file activity stops
    pub debouncer: Debouncer<F>,
}

impl<'a, F> Watcher<'a, F>
where
    F: FnMut(EventContext) + Send + 'static,
{
    /// Creates a new `Watcher` instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration settings including watch directory and timing
    /// * `debouncer_cb` - Callback function that will be executed after the debounce period
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use watchers::{Watcher, Config, git::handle_event};
    /// use std::path::PathBuf;
    ///
    /// let config = Config {
    ///     watch_dir: PathBuf::from("/my/project"),
    ///     commit_delay_secs: 5,
    ///     auto_push: true,
    ///     config_path: None,
    /// };
    ///
    /// let watcher = Watcher::new(&config, handle_event);
    /// ```
    pub fn new(config: &'a Config, debouncer_cb: F) -> Self {
        let debouncer = Debouncer::new(debouncer_cb, Duration::from_secs(config.commit_delay_secs as u64));
        Self { config, debouncer }
    }

    /// Triggers the debouncer with current configuration.
    ///
    /// This method creates an `EventContext` from the current configuration
    /// and passes it to the debouncer. The debouncer will delay execution
    /// of the callback until the configured quiet period has elapsed.
    pub fn trigger_debouncer(&mut self) {
        debug!("triggering debouncer");
        let context = EventContext {
            repo_path: self.config.watch_dir.clone(),
            config: self.config.clone(),
        };
        self.debouncer.on_event(context);
    }
}

/// Starts watching a directory for file changes and automatically creates git commits.
///
/// This function sets up file system monitoring using the `notify` crate and processes
/// file modification events through the provided watcher's debouncer. It runs indefinitely
/// until an error occurs or the process is terminated.
///
/// # Arguments
///
/// * `watcher` - A mutable reference to a configured `Watcher` instance
///
/// # Returns
///
/// Returns `Ok(())` if watching completes successfully (which should never happen in normal operation),
/// or an error if the file system watcher fails to initialize or encounters a fatal error.
///
/// # Errors
///
/// This function will return an error if:
/// - The file system watcher cannot be created
/// - The watch directory cannot be accessed
/// - The underlying file system monitoring fails
///
/// # Example
///
/// ```rust,no_run
/// use watchers::{Watcher, Config, git::handle_event, watch_repo};
/// use std::path::PathBuf;
///
/// let config = Config {
///     watch_dir: PathBuf::from("/path/to/watch"),
///     commit_delay_secs: 3,
///     auto_push: false,
///     config_path: None,
/// };
///
/// let mut watcher = Watcher::new(&config, handle_event);
///
/// // This blocks indefinitely, watching for changes
/// watch_repo(&mut watcher).expect("Failed to start watching");
/// ```
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

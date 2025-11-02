use crate::{
    config::{Config, get_watchers_config_dir},
    debouncer::Debouncer,
    file_utils::was_modification,
    git::{EventContext, handle_event},
    systemd::SystemdContext,
};

use anyhow::{Context, Result};
use git2::Repository;
use inquire::{Confirm, Text};
use log::debug;
use notify::{Event, RecursiveMode};
use std::{
    fs::{self},
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

pub struct Watcher<'a, F> {
    pub config: &'a Config,
    pub debouncer: Debouncer<F>,
}

impl<'a, F> Watcher<'a, F>
where
    F: FnMut(EventContext) + Send + 'static,
{
    pub fn new(config: &'a Config, debouncer_cb: F) -> Self {
        let debouncer = Debouncer::new(
            debouncer_cb,
            Duration::from_secs(config.commit_delay_secs as u64),
        );
        Self { config, debouncer }
    }

    pub fn trigger_debouncer(&mut self) {
        let context = EventContext {
            repo_path: self.config.watch_dir.clone(),
            config: self.config.clone(),
        };
        self.debouncer.on_event(context);
    }
}

fn is_git_file(paths: &[impl AsRef<Path>]) -> Result<bool> {
    if paths.is_empty() {
        return Ok(false);
    }

    // If any path contains .git as a component, it's a git internal file
    // We should ignore these files regardless of git-ignore status
    Ok(paths
        .iter()
        .any(|p| p.as_ref().components().any(|c| c.as_os_str() == ".git")))
}

pub fn watch_repo<F>(watcher: &mut Watcher<F>) -> Result<()>
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
                    && !is_git_file(&ev.paths)?
                    && !is_git_ignored(&ev.paths)?
                {
                    debug!("got valid modification: {:?} - triggering debouncer", ev);
                    watcher.trigger_debouncer();
                }
            }
        }
    }
}

fn is_git_ignored<P: AsRef<Path>>(paths: &[P]) -> Result<bool> {
    if paths.is_empty() {
        return Ok(false);
    }

    let repo = Repository::discover(paths[0].as_ref().parent().unwrap())?;
    for p in paths {
        let rel_path = p.as_ref().strip_prefix(repo.workdir().unwrap())?;
        if repo.is_path_ignored(rel_path)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn get_watcher_config(name: &str) -> Result<Config> {
    let path = Config::get_watcher_config_path(name);
    anyhow::ensure!(path.is_file(), "Could not find config for '{}'", name);
    Config::from_file(path)
}

pub async fn start_watcher(name: &str) -> Result<()> {
    let systemd_ctx = SystemdContext::new().await?;

    systemd_ctx.start_and_enable_service(name).await?;
    Ok(())
}

pub async fn create_watcher(name: &str) -> Result<()> {
    let path_input = Text::new("Path to directory to watch:")
        .prompt()
        .context("Failed to read input")?;
    let path = PathBuf::from(shellexpand::tilde(&path_input).to_string());

    anyhow::ensure!(
        path.is_dir(),
        "Path '{}' is not a directory",
        path.display()
    );

    let config = Config::new(name, &path);
    let config_path = Config::get_watcher_config_path(name);

    let mut should_overwrite_config: bool = true;
    if config_path.exists() {
        anyhow::ensure!(
            config_path.is_file(),
            format!(
                "'{}' is not a regular file.",
                config_path.as_os_str().display()
            )
        );

        should_overwrite_config = Confirm::new(&format!(
            "'{}' already exists, do you want to overwrite it?",
            config_path.as_os_str().display()
        ))
        .with_default(false)
        .prompt()?;
    }

    if !should_overwrite_config {
        return Ok(());
    }

    if let Some(parent) = &config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&config_path, config.dump()?).context(format!(
        "Failed to write config to: {}",
        &config_path.as_os_str().display()
    ))?;

    let systemd_ctx = SystemdContext::new().await?;
    systemd_ctx.start_and_enable_service(name).await?;

    Ok(())
}

pub async fn stop_watcher(name: &str) -> Result<()> {
    let systemd_ctx = SystemdContext::new().await?;
    systemd_ctx.stop_and_disable_service(name).await?;

    Ok(())
}

pub fn delete_watcher(name: &str) -> Result<()> {
    let config_path = Config::get_watcher_config_path(name);
    anyhow::ensure!(config_path.is_file(), "Couldn't find watcher '{}'", name);
    fs::remove_file(config_path)?;
    Ok(())
}

pub fn list_watchers() -> Result<()> {
    println!("Watchers:");
    let config_dir = get_watchers_config_dir();
    for file in fs::read_dir(config_dir)? {
        let path = file?.path();
        if !path.is_file() {
            continue;
        }

        println!(
            "  {}",
            path.file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to read config directory"))?
        );
    }

    Ok(())
}

pub async fn run_daemon(name: &str) -> Result<()> {
    let config = get_watcher_config(name)?;
    debug!(
        "Config:\n{}",
        config.dump().unwrap_or("failed to read config".to_string())
    );

    let mut watcher = Watcher::new(&config, |context| {
        handle_event(context);
    });

    watch_repo(&mut watcher)?;

    anyhow::bail!("Should never finish watching");
}

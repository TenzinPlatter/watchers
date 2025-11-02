use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use zbus::Connection;
use zbus_systemd::systemd1::ManagerProxy;

pub struct SystemdContext<'a> {
    _conn: Connection,
    manager: ManagerProxy<'a>,
}

fn get_unit_name(name: &str) -> String {
    format!("watchers@{name}.service")
}

impl<'a> SystemdContext<'a> {
    pub async fn new() -> Result<Self> {
        let conn = Connection::session().await?;
        let manager = ManagerProxy::new(&conn).await?;

        Ok(Self {
            _conn: conn,
            manager,
        })
    }

    pub async fn stop_and_disable_service(&self, name: &str) -> Result<()> {
        let unit_name = get_unit_name(name);

        self.manager
            .stop_unit(unit_name.clone(), "replace".to_string())
            .await
            .context("Failed to stop systemd service")?;

        self.manager
            .disable_unit_files(vec![unit_name.clone()], false)
            .await
            .context("Failed to enable systemd service")?;

        self.manager.reload().await?;

        Ok(())
    }

    pub async fn start_and_enable_service(&self, name: &str) -> Result<()> {
        let template_unit_path = get_systemd_unit_path();
        if !template_unit_path.is_file() {
            fs::write(template_unit_path, get_template_unit_contents())?;
        }

        let unit_name = get_unit_name(name);

        self.manager
            .start_unit(unit_name.clone(), "replace".to_string())
            .await
            .context("Failed to start systemd service")?;

        self.manager
            .enable_unit_files(vec![unit_name.clone()], false, true)
            .await
            .context("Failed to enable systemd service")?;

        self.manager.reload().await?;

        Ok(())
    }
}

fn get_systemd_unit_path() -> PathBuf {
    let proj_dir = ProjectDirs::from("", "", "").unwrap();
    let config_dir = proj_dir.config_dir();

    PathBuf::from(format!(
        "{}/systemd/user/watchers@.service",
        config_dir.display()
    ))
}

fn get_template_unit_contents() -> String {
    let default_exe_path = "/usr/bin/local/watchers";
    let exe_path = env::current_exe().unwrap_or_else(|_| default_exe_path.into());
    format!(
        include_str!("../assets/templates/watchers@.service"),
        exe_path.as_os_str().to_str().unwrap_or(default_exe_path)
    )
}

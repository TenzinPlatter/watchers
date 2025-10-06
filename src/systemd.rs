use std::path::PathBuf;

use zbus::Connection;
use zbus_systemd::systemd1::ManagerProxy;

use crate::Config;

pub struct SystemdContext<'a> {
    _conn: Connection,
    manager: ManagerProxy<'a>,
    config: ServiceConfig,
}

struct ServiceConfig {
    name: String,
    unit_name: String,
    watch_path: PathBuf,
}

impl<'a> SystemdContext<'a> {
    pub async fn new(watcher_config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::system().await?;
        let manager = ManagerProxy::new(&conn).await?;

        let config = ServiceConfig {
            name: watcher_config.name.clone(),
            unit_name: format!("watchers-{}.service", watcher_config.name),
            watch_path: watcher_config.watch_dir.clone(),
        };

        Ok(Self {
            _conn: conn,
            manager,
            config,
        })
    }

    fn generate_unit_file(&self) -> String {
        format!(
            include_str!("../assets/templates/watcher.service"),
            self.config.name,
            self.config.watch_path.display()
        )
    }

    pub async fn stop_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.manager
            .stop_unit(self.config.unit_name.clone(), "replace".to_string())
            .await?;

        Ok(())
    }

    pub async fn create_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let unit_content = self.generate_unit_file();

        std::fs::write(
            format!("/etc/systemd/{}", self.config.unit_name),
            unit_content,
        )?;

        Ok(())
    }

    pub async fn start_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.manager
            .start_unit(self.config.unit_name.clone(), "replace".to_string())
            .await?;

        Ok(())
    }
}

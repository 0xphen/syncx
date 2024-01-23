use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use super::errors::SynxClientError;

#[derive(Debug)]
pub struct Context {
    pub app_config: AppConfig,
    pub path: PathBuf,
}

impl Context {
    pub fn new(app_config: AppConfig, path: PathBuf) -> Self {
        Self { app_config, path }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct AppConfig {
    id: String,
    password: String,
    pub jwt: String,
    pub merkle_tree_root: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            merkle_tree_root: String::new(),
            id: String::new(),
            jwt: String::new(),
            password: String::new(),
        }
    }
}

impl AppConfig {
    pub fn get_config_path() -> Result<PathBuf, SynxClientError> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "Synx", "Client") {
            let config_dir = proj_dirs.config_dir();

            std::fs::create_dir_all(config_dir)
                .map_err(|_| SynxClientError::ConfigDirectoryCreationError)?;

            Ok(config_dir.join("config.json"))
        } else {
            Err(SynxClientError::ConfigDirectoryCreationError)
        }
    }

    pub fn write(&self, path: &PathBuf) -> Result<(), SynxClientError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .map_err(|_| SynxClientError::ConfigFileWriteError)?;

        serde_json::to_writer(file, self).map_err(|_| SynxClientError::ConfigFileWriteError)?;

        Ok(())
    }

    pub fn read(path: &PathBuf) -> std::io::Result<AppConfig> {
        let file = File::open(path)?;
        let config = serde_json::from_reader(file)?;
        Ok(config)
    }

    pub fn set_merkle_root(&mut self, root: String) {
        self.merkle_tree_root = root;
    }

    pub fn set_jwt(&mut self, jwt: String) {
        self.jwt = jwt;
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_password(&mut self, password: String) {
        self.password = password;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_test() {
        let path = AppConfig::get_config_path().unwrap();

        // Write when config is empty
        let mut app_config = AppConfig::default();
        app_config.write(&path);

        let mut config = AppConfig::read(&path).unwrap();
        assert!(config == AppConfig::default());

        // Write when config is not empty
        app_config.merkle_tree_root = "merkle_tree_root".to_string();
        app_config.id = "abcd".to_string();
        app_config.jwt = "jwt".to_string();
        app_config.password = "password".to_string();

        app_config.write(&path);
        config = AppConfig::read(&path).unwrap();

        assert!(config == app_config);
    }
}

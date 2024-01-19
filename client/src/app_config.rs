use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::errors::SynxClientError;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct AppConfig {
    pub merkle_tree_root: String,
    id: String,
    jwt: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            merkle_tree_root: String::new(),
            id: String::new(),
            jwt: String::new(),
        }
    }
}

impl AppConfig {
    fn get_config_path() -> Result<PathBuf, SynxClientError> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "Synx", "Client") {
            let config_dir = proj_dirs.config_dir();

            std::fs::create_dir_all(config_dir)
                .map_err(|_| SynxClientError::ConfigDirectoryCreationError)?;

            Ok(config_dir.join("config.json"))
        } else {
            Err(SynxClientError::ConfigDirectoryCreationError)
        }
    }

    fn write_config(&self, path: &PathBuf) -> Result<(), SynxClientError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            // .truncate(true)
            .open(path)
            .map_err(|_| SynxClientError::ConfigFileWriteError)?;

        serde_json::to_writer(file, self).map_err(|_| SynxClientError::ConfigFileWriteError)?;

        Ok(())
    }

    fn read_config(path: &PathBuf) -> std::io::Result<AppConfig> {
        let file = File::open(path)?;
        let config = serde_json::from_reader(file)?;
        Ok(config)
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
        app_config.write_config(&path);

        let mut config = AppConfig::read_config(&path).unwrap();
        assert!(config == AppConfig::default());

        // Write when config is not empty
        app_config.merkle_tree_root = "merkle_tree_root".to_string();
        app_config.id = "abcd".to_string();
        app_config.jwt = "jwt".to_string();

        app_config.write_config(&path);
        config = AppConfig::read_config(&path).unwrap();

        assert!(config == app_config);
    }
}

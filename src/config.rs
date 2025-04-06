use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub ask: bool,
    pub default_instance: Option<String>,
    pub instances: HashMap<String, PathBuf>,
}


impl Config {
    pub fn load() -> Self {
        let path = config_dir().unwrap().join("fmods/config.toml");

        toml::from_str(&match std::fs::read_to_string(path) {
            Ok(str) => str,
            Err(_) => return Self::default()
        }).unwrap_or(Self::default())
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = config_dir().unwrap().join("fmods/config.toml");

        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?
            .write_all(toml::to_string(&self)?.as_bytes())?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ask: true,
            default_instance: None,
            instances: HashMap::new(),
        }
    }
}
use std::env::var;
use std::fs::File;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use std::io::prelude::*;
use std::error::Error;


#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub db_path: PathBuf,
    pub feeds_path: PathBuf,

    #[serde(default)]
    pub update_rate: u64,

    #[serde(default)]
    pub keywords: Vec<String>
}

impl Default for Config {
    fn default() -> Config {
        Config {
            update_rate: 1200,
            db_path: config_path("rssrs.db"),
            feeds_path: config_path("feeds.txt"),
            keywords: Vec::new()
        }
    }
}

impl Config {
    pub fn load() -> Result<Config, Box<dyn Error>> {
        let path = config_path("config.toml");
        if path.exists() {
            let mut content = String::new();
            File::open(path)?.read_to_string(&mut content)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }
}

fn config_path<P>(path: P) -> PathBuf where P: AsRef<Path> {
    let home = match var("HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => PathBuf::from(".")
    };
    home.join(".config/rssrs").join(path)
}

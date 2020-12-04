use std::env::var;
use std::fs::File;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use std::io::prelude::*;


#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub db_path: PathBuf,
    pub feeds_path: PathBuf,
    pub update_rate: u64,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            update_rate: 1200,
            db_path: config_path("rssrs.db"),
            feeds_path: config_path("feeds.txt")
        }
    }
}

impl Config {
    pub fn load() -> Config {
        let path = config_path("config.toml");
        if path.exists() {
            let mut file = File::open(path).expect("Couldn't open config");
            let mut content = String::new();
            file.read_to_string(&mut content).expect("Error reading config");
            toml::from_str(&content).expect("Error while parsing config toml")
        } else {
            Config::default()
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

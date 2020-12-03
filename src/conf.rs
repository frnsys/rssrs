use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::time::Duration;
use termion::event::Key;



#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: String,
    pub feeds_path: String,
    pub keys: KeyBindings,
    pub update_rate: Duration,
}


impl Default for Config {
    fn default() -> Config {
        Config {
            keys: KeyBindings::default(),
            update_rate: Duration::from_millis(25000),
            db_path: "data/rsrss.db".to_string(),
            feeds_path: "data/feeds.txt".to_string()
        }
    }
}

impl Config {
    /*
     * Feeds file: one feed per line in the format:
     * <url> <comma-delimited tags>
     */
    pub fn load_feeds(&self) -> impl Iterator<Item=(String, Vec<String>)> {
        let file = File::open(&self.feeds_path).unwrap();

        BufReader::new(file).lines().filter_map(Result::ok).map(|line| {
            let mut split = line.splitn(2, ' ');
            let url = split.next().unwrap().to_string();
            let tags = split.next().unwrap_or("").split(",").map(|s| s.to_string()).collect();
            (url, tags)
        })
    }
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub exit_key: Key,
}

impl Default for KeyBindings {
    fn default() -> KeyBindings {
        KeyBindings {
            exit_key: Key::Char('q'),
        }
    }
}

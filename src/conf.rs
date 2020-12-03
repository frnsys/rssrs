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

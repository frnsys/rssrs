use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;

use super::sync::update;
use super::db::{Database, Item};
use super::conf::load_feeds;


pub enum Event<I> {
    Input(I),
    Updating,
    Updated,
}

/// A small event handler that wrap termion input and update events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    input_handle: thread::JoinHandle<()>,
    ignore_exit_key: Arc<AtomicBool>,
    update_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub update_rate: Duration,
    pub db_path: Path,
    pub feeds_path: Path
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            update_rate: Duration::from_millis(25000),
            db_path: "data/rsrss.db".to_string(),
            feeds_path: "data/feeds.txt".to_string()
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let ignore_exit_key = Arc::new(AtomicBool::new(false));
        let input_handle = {
            let tx = tx.clone();
            let ignore_exit_key = ignore_exit_key.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if let Err(err) = tx.send(Event::Input(key)) {
                            eprintln!("{}", err);
                            return;
                        }
                        if !ignore_exit_key.load(Ordering::Relaxed) && key == config.exit_key {
                            return;
                        }
                    }
                }
            })
        };

        let conf = config.clone();
        let update_handle = {
            thread::spawn(move || loop {
                if tx.send(Event::Updating).is_err() {
                    break;
                }

                let db = Database::new(&conf.db_path);
                for (feed_url, _tags) in load_feeds(&conf.feeds_path) {
                    update(&feed_url, &db).unwrap();
                }

                if tx.send(Event::Updated).is_err() {
                    break;
                }
                thread::sleep(config.update_rate);
            })
        };
        Events {
            rx,
            ignore_exit_key,
            input_handle,
            update_handle,
        }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn disable_exit_key(&mut self) {
        self.ignore_exit_key.store(true, Ordering::Relaxed);
    }

    pub fn enable_exit_key(&mut self) {
        self.ignore_exit_key.store(false, Ordering::Relaxed);
    }
}


use std::io;
use std::sync::{
    mpsc,
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

use termion::event::Key;
use termion::input::TermRead;

use super::db::Database;
use super::sync::update;
use super::conf::Config;
use super::util::load_feeds;


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


impl Events {
    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let ignore_exit_key = Arc::new(AtomicBool::new(false));
        let input_handle = {
            let tx = tx.clone();
            let ignore_exit_key = ignore_exit_key.clone();
            let exit_key = config.keys.exit_key;
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if let Err(err) = tx.send(Event::Input(key)) {
                            eprintln!("{}", err);
                            return;
                        }
                        if !ignore_exit_key.load(Ordering::Relaxed) && key == exit_key {
                            return;
                        }
                    }
                }
            })
        };

        let update_handle = {
            thread::spawn(move || loop {
                if tx.send(Event::Updating).is_err() {
                    break;
                }

                let db = Database::new(&config.db_path);
                for (feed_url, _tags) in load_feeds(&config.feeds_path) {
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


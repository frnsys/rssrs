mod db;
mod sync;
mod conf;

use self::sync::update;
use self::db::Database;
use std::{thread, time};
use self::conf::load_feeds;


fn main() {
    let db_path = "/tmp/rsrss.db";
    let feeds_path = "/tmp/feeds.txt";
    let update_interval = 10 * 60 * 1000; // ms
    let handle = thread::spawn(move || {
        let update_duration = time::Duration::from_millis(update_interval);
        loop {
            println!("updating...");
            let db = Database::new(&db_path);
            for (feed_url, _tags) in load_feeds(&feeds_path) {
                println!("{:?}", feed_url);
                update(&feed_url, &db).unwrap();
            }
            println!("done updating");
            thread::sleep(update_duration);
        }
    });
    handle.join().unwrap();
}

use std::path::Path;
use rusqlite::{params, Connection, Result};

#[derive(Debug)]
pub struct Item {
    pub read: bool,
    pub channel: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub published_at: Option<i64>,
    pub description: Option<String>
}

pub struct Database {
    conn: Connection
}

impl Database {
    pub fn new<P>(path: P) -> Database where P: AsRef<Path> {
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS item (
                      url             TEXT PRIMARY KEY,
                      read            INTEGER DEFAULT 0,
                      channel         TEXT,
                      title           TEXT,
                      published_at    INTEGER,
                      description     TEXT
                      )",
            params![],
        ).unwrap();

        Database {
            conn: conn
        }
    }

    pub fn add_item(&self, item: &Item) -> Result<()> {
        // Ignore unique constraint conflicts
        self.conn.execute(
            "INSERT OR IGNORE INTO item (url, channel, title, published_at, description) VALUES (?, ?, ?, ?, ?)",
            params![item.url, item.channel, item.title, item.published_at, item.description],
        )?;
        Ok(())
    }

    pub fn set_item_read(&self, item: &Item, read: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE item SET read=? WHERE url == ?",
            params![read, item.url],
        )?;
        Ok(())
    }

    pub fn get_channel_items(&self, channel: &str) -> Result<Vec<Item>> {
        let mut stmt = self.conn.prepare("SELECT * FROM item WHERE channel == ?")?;
        let items = stmt.query_map(&[channel], |row| {
            Ok(Item {
                url: row.get(0)?,
                read: row.get(1)?,
                channel: row.get(2)?,
                title: row.get(3)?,
                published_at: row.get(4)?,
                description: row.get(5)?,
            })
        })?.filter_map(Result::ok).collect();
        Ok(items)
    }
}

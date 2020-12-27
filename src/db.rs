use rusqlite::{params, Connection, Result};
use std::fs::{create_dir_all, File};
use std::path::Path;

#[derive(Debug)]
pub struct Item {
    pub read: bool,
    pub starred: bool,
    pub feed: String,
    pub retrieved_at: i64,
    pub title: Option<String>,
    pub url: Option<String>,
    pub published_at: Option<i64>,
    pub description: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new<P>(path: P) -> Database
    where
        P: AsRef<Path>,
    {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            create_dir_all(path_buf.parent().unwrap()).unwrap();
            File::create(&path).unwrap();
        }
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS item (
                      url             TEXT PRIMARY KEY,
                      read            INTEGER DEFAULT 0,
                      starred         INTEGER DEFAULT 0,
                      feed            TEXT,
                      title           TEXT,
                      published_at    INTEGER,
                      retrieved_at    INTEGER,
                      description     TEXT
                      )",
            params![],
        )
        .unwrap();

        Database { conn }
    }

    pub fn add_item(&self, item: &Item) -> Result<()> {
        // Ignore unique constraint conflicts
        self.conn.execute(
            "INSERT OR IGNORE INTO item (url, feed, title, published_at, retrieved_at, description) VALUES (?, ?, ?, ?, ?, ?)",
            params![item.url, item.feed, item.title, item.published_at, item.retrieved_at, item.description],
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

    pub fn set_item_starred(&self, item: &Item, starred: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE item SET starred=? WHERE url == ?",
            params![starred, item.url],
        )?;
        Ok(())
    }

    pub fn get_feed_items(&self, feed: &str) -> Result<Vec<Item>> {
        let mut stmt = self.conn.prepare("SELECT * FROM item WHERE feed == ?")?;
        let items = stmt
            .query_map(&[feed], |row| {
                Ok(Item {
                    url: row.get(0)?,
                    read: row.get(1)?,
                    starred: row.get(2)?,
                    feed: row.get(3)?,
                    title: row.get(4)?,
                    published_at: row.get(5)?,
                    retrieved_at: row.get(6)?,
                    description: row.get(7)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(items)
    }

    pub fn last_update(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT max(retrieved_at) FROM item", params![], |row| {
                row.get(0)
            })
    }
}

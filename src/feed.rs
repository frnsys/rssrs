use rss::Channel;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use chrono::{DateTime, Utc};
use rusqlite::Result;
use html2md::parse_html;
use super::db::Item;
use std::error::Error;

const MAX_AGE: i64 = 60*60*24*182; // about 6 months

pub struct Feed {
    pub url: String,
    pub title: String,
    pub tags: Vec<String>
}

/*
 * Feeds file: one feed per line in the format:
 * <url> <comma-delimited tags>
 */
pub fn load_feeds<P>(path: P) -> impl Iterator<Item=Feed> where P: AsRef<Path> {
    let file = File::open(&path).unwrap_or_else(|_| {
        File::create(&path).unwrap();
        File::open(&path).unwrap()
    });

    BufReader::new(file).lines().filter_map(Result::ok)
        .filter(|line| !line.starts_with('#')).map(|line| {
            let mut split = line.splitn(3, '|');
            let url = split.next().unwrap().to_string();
            let title = split.next().unwrap().to_string();
            let tags = split.next().unwrap_or("").split(',').map(|s| s.to_string()).collect();
            Feed { url, title, tags }
        })
}


pub async fn get_items(feed_url: String) -> Result<Vec<Item>, Box<dyn Error>> {
    let content = reqwest::get(&feed_url)
        .await?
        .bytes()
        .await?;
    let feed = Channel::read_from(&content[..])?;
    let now = Utc::now().timestamp();
    let mut items = Vec::new();
    for it in feed.items() {
        let item = Item {
            read: false,
            starred: false,
            feed: feed_url.clone(),
            title: it.title().map(Into::into),
            url: it.link().map(Into::into),
            retrieved_at: now,
            published_at: match it.pub_date().map(Into::into) {
                Some(pub_date) => {
                    let dt = DateTime::parse_from_rfc2822(pub_date).unwrap();
                    Some(dt.timestamp())
                },
                None => None
            },
            description: match it.description().map(Into::into) {
                Some(desc) => Some(parse_html(desc)),
                None => None
            },
        };

        // Only save items above a certain age
        if let Some(published) = item.published_at {
            if published > now - MAX_AGE {
                items.push(item);
            }
        }
    }
    Ok(items)
}

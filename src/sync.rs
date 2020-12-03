use rss::Channel;
use rusqlite::Result;
use super::db::{Database, Item};
use html2md::parse_html;

pub fn update(channel_url: &str, db: &Database) -> Result<()> {
    let channel = Channel::from_url(channel_url).unwrap();
    let pub_date = channel.pub_date();
    // let title = channel.title();
    for it in channel.items() {
        let item = Item {
            read: false,
            channel: channel_url.to_string(),
            title: it.title().map(Into::into),
            url: it.link().map(Into::into),
            published_at: it.pub_date().map(Into::into),
            description: match it.description().map(Into::into) {
                Some(desc) => Some(parse_html(desc)),
                None => None
            },
        };
        db.add_item(&item)?
    }
    Ok(())
}

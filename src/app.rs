use webbrowser;
use std::path::{Path, PathBuf};
use chrono::{TimeZone, Local, Utc};
use super::db::{Database, Item};
use super::util::{StatefulTable};
use super::feeds::{Feed, load_feeds};
use std::collections::HashMap;
use regex::{Regex, RegexBuilder};

pub enum InputMode {
    Normal,
    Search,
}

pub struct Filter {
    pub read: Option<bool>,
    pub starred: Option<bool>,
    pub feeds: Vec<String>,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
}

impl Default for Filter {
    fn default() -> Filter {
        Filter {
            read: Some(false),
            starred: None,
            feeds: vec![],
            keywords: vec![],
            tags: vec![]
        }
    }
}

impl Filter {
    pub fn filter_feed(&self, feed: &Feed) -> bool {
        (self.feeds.len() == 0 || self.feeds.contains(&feed.url))
            && (self.tags.len() == 0 || self.tags.iter().any(|tag| feed.tags.contains(tag)))
    }

    pub fn filter_item(&self, item: &Item) -> bool {
        (match self.read {
            Some(read) => item.read == read,
            None => true
        }) && (match self.starred {
            Some(starred) => item.starred == starred,
            None => true
        }) && (self.keywords.len() == 0 || self.keywords.iter().any(|kw| match &item.title {
            Some(title) => title.contains(kw),
            None => false
        }))
    }
}

pub enum Status {
    Idle,
    Updating
}

pub struct App {
    db: Database,
    feeds_path: PathBuf,

    pub focus_reader: bool,
    pub status: Status,
    pub input_mode: InputMode,
    pub last_updated: i64,

    pub filter: Filter,
    pub items: Vec<Item>,
    pub feeds: HashMap<String, Feed>,
    pub table: StatefulTable,

    pub search_results: Vec<usize>,
    pub search_input_raw: String,
    pub search_input: Option<Regex>,
    pub search_query: Option<Regex>,

    pub reader_scroll: u16,
    pub marked: Vec<usize>,
}

impl App {
    pub fn new<P>(db_path: P, feeds_path: P) -> App where P: AsRef<Path> {
        App {
            db: Database::new(db_path),
            feeds_path: feeds_path.as_ref().to_path_buf(),

            input_mode: InputMode::Normal,
            focus_reader: false,
            status: Status::Idle,
            last_updated: 0,

            filter: Filter::default(),
            items: Vec::new(),
            feeds: HashMap::default(),
            table: StatefulTable::new(),

            search_input: None,
            search_query: None,
            search_results: Vec::new(),
            search_input_raw: String::new(),

            reader_scroll: 0,
            marked: Vec::new(),
        }
    }

    // Load items according to filter
    pub fn _load_items(&mut self) -> Vec<Item> {
        // Also store feeds for referencing later
        for feed in load_feeds(&self.feeds_path) {
            self.feeds.insert(feed.url.clone(), feed);
        }

        let feeds = load_feeds(&self.feeds_path).filter(|f| self.filter.filter_feed(f));

        // TODO why do I need both flat map and flatten?
        let mut items: Vec<Item> = feeds.flat_map(|feed| {
                self.db.get_feed_items(&feed.url).ok()
            }).flatten().filter(|i| self.filter.filter_item(i)).collect();

        // Most recent first
        items.sort_by_cached_key(|i| match i.published_at {
            Some(ts) => -ts,
            None => 0
        });

        items
    }

    pub fn load_new_items(&mut self) {
        let mut new: Vec<Item> = self._load_items().into_iter().filter(|item| item.retrieved_at > self.last_updated).collect();
        self.last_updated = Utc::now().timestamp();

        // Add and sort recent first
        self.items.append(&mut new);
        self.items.sort_by_cached_key(|i| match i.published_at {
            Some(ts) => -ts,
            None => 0
        });

        self.update_items_table();
    }

    pub fn load_items(&mut self) {
        self.items = self._load_items();
        self.last_updated = self.db.last_update().unwrap();
        self.update_items_table();
    }

    pub fn update_items_table(&mut self) {
        // Load item data into table
        self.table.set_items(self.items.iter().map(|i| {
            let pub_date = match i.published_at {
                Some(ts) => Local.timestamp(ts, 0).format("%m/%d/%y %H:%M").to_string(),
                None => "<no pub date>".to_string()
            };

            vec![
                i.title.as_deref().unwrap_or("<no title>").to_string(),
                pub_date,
            ]
        }).collect());
    }

    pub fn mark_selected_read(&mut self) {
        match self.table.state.selected() {
            Some(i) => {
                self.items[i].read = true;
                self.db.set_item_read(&self.items[i], true);
            },
            None => {}
        }
    }

    pub fn mark_selected_unread(&mut self) {
        match self.table.state.selected() {
            Some(i) => {
                self.items[i].read = false;
                self.db.set_item_read(&self.items[i], false);
            },
            None => {}
        }
    }

    pub fn toggle_selected_read(&mut self) {
        match self.table.state.selected() {
            Some(i) => {
                self.items[i].read = !self.items[i].read;
                self.db.set_item_read(&self.items[i], self.items[i].read);
            },
            None => {}
        }
    }

    pub fn build_query(&self, query: &String) -> Regex {
        let regex = format!(r"({})", query);
        RegexBuilder::new(&regex).case_insensitive(true).build().expect("Invalid regex")
    }

    pub fn execute_search(&mut self, query: &Regex) {
        self.search_results = self.items.iter().enumerate().filter(|(_, item)| {
            match &item.title {
                Some(title) => query.is_match(title),
                None => false
            }
        }).map(|i| i.0).collect();
    }

    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
    }

    pub fn end_search(&mut self) {
        self.search_input_raw.clear();
        self.input_mode = InputMode::Normal;
    }

    pub fn scroll_items_up(&mut self) {
        self.table.previous();
        self.mark_selected_read();
        self.reset_reader_scroll();
    }

    pub fn scroll_items_down(&mut self) {
        self.table.next();
        self.mark_selected_read();
        self.reset_reader_scroll();
    }

    pub fn page_items_up(&mut self) {
        self.table.jump_backward(5);
        self.mark_selected_read();
        self.reset_reader_scroll();
    }

    pub fn page_items_down(&mut self) {
        self.table.jump_forward(5);
        self.mark_selected_read();
        self.reset_reader_scroll();
    }

    pub fn reset_reader_scroll(&mut self) {
        self.reader_scroll = 0;
    }

    pub fn scroll_reader_up(&mut self) {
        if self.reader_scroll > 0 {
            self.reader_scroll -= 1;
        }
    }

    pub fn scroll_reader_down(&mut self) {
        self.reader_scroll += 1;
    }

    pub fn toggle_focus_reader(&mut self) {
        self.focus_reader = !self.focus_reader;
    }

    pub fn open_selected(&self) {
        match self.table.state.selected() {
            Some(i) => {
                match &self.items[i].url {
                    Some(url) => {
                        webbrowser::open(&url);
                    },
                    None => {}
                }
            },
            None => {}
        };
    }

    pub fn open_marked(&self) {
        for i in &self.marked {
            match &self.items[*i].url {
                Some(url) => {
                    webbrowser::open(&url);
                },
                None => {}
            }
        }
    }

    pub fn jump_to_next_result(&mut self) {
        if self.search_results.len() > 0 {
            match self.table.state.selected() {
                Some(i) => {
                    if i >= *self.search_results.last().unwrap() {
                        self.table.state.select(Some(self.search_results[0]));
                    } else {
                        for si in &self.search_results {
                            if *si > i {
                                self.table.state.select(Some(*si));
                                break;
                            }
                        }
                    }
                },
                None => {
                    self.table.state.select(Some(self.search_results[0]));
                }
            }
        }
    }

    pub fn jump_to_prev_result(&mut self) {
        if self.search_results.len() > 0 {
            match self.table.state.selected() {
                Some(i) => {
                    if i <= self.search_results[0] {
                        let last = self.search_results.last().unwrap();
                        self.table.state.select(Some(*last));
                    } else {
                        for si in self.search_results.iter().rev() {
                            if *si < i {
                                self.table.state.select(Some(*si));
                                break;
                            }
                        }
                    }
                },
                None => {
                    self.table.state.select(Some(self.search_results[0]));
                }
            }
        }
    }

    pub fn clear_marked(&mut self) {
        self.marked.clear();
    }

    pub fn toggle_selected_mark(&mut self) {
        match self.table.state.selected() {
            Some(i) => {
                if self.marked.contains(&i) {
                    self.marked.retain(|i_| i_ != &i);
                } else {
                    self.marked.push(i);
                }
            },
            None => {}
        }
    }

    pub fn toggle_selected_star(&mut self) {
        match self.table.state.selected() {
            Some(i) => {
                self.items[i].starred = !self.items[i].starred;
                self.db.set_item_starred(&self.items[i], self.items[i].starred);
            },
            None => {}
        }
    }

    pub fn toggle_read_filter(&mut self) {
        // All => Unread => Read
        self.filter.read = match self.filter.read {
            Some(read) => {
                if read {
                    None
                } else {
                    Some(true)
                }
            },
            None => Some(false)
        };
        self.load_items();
    }

    pub fn toggle_starred_filter(&mut self) {
        // All => Starred => Unstarred
        self.filter.starred = match self.filter.starred {
            Some(starred) => {
                if !starred {
                    None
                } else {
                    Some(false)
                }
            },
            None => Some(true)
        };
        self.load_items();
    }
}

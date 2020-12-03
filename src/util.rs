use regex::Regex;
use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use tui::widgets::TableState;
use std::io::prelude::*;

// https://github.com/fdehau/tui-rs/blob/master/examples/table.rs
pub struct StatefulTable {
    pub state: TableState,
    pub items: Vec<Vec<String>>,
}

impl StatefulTable {
    pub fn new() -> StatefulTable {
        StatefulTable {
            state: TableState::default(),
            items: vec![],
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn jump_forward(&mut self, n: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                usize::min(i + n, self.items.len() - 1)
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn jump_backward(&mut self, n: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if n > i {
                    0
                } else {
                    i - n
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn set_items(&mut self, items: Vec<Vec<String>>) {
        self.items = items;
    }
}


// Split a string on a regex, keeping the matching parts
// and marking which parts are the matched ones
pub fn split_keep<'a>(r: &Regex, text: &'a str) -> Vec<(&'a str, bool)> {
    let mut result = Vec::new();
    let mut last = 0;
    for mat in r.find_iter(text) {
        let index = mat.start();
        let matched = mat.as_str();
        if last != index {
            result.push((&text[last..index], false));
        }
        result.push((matched, true));
        last = index + matched.len();
    }
    if last < text.len() {
        result.push((&text[last..], false));
    }
    result
}

/*
 * Feeds file: one feed per line in the format:
 * <url> <comma-delimited tags>
 */
pub fn load_feeds<P>(path: P) -> impl Iterator<Item=(String, Vec<String>)> where P: AsRef<Path> {
    let file = File::open(&path).unwrap();

    BufReader::new(file).lines().filter_map(Result::ok).map(|line| {
        let mut split = line.splitn(2, ' ');
        let url = split.next().unwrap().to_string();
        let tags = split.next().unwrap_or("").split(",").map(|s| s.to_string()).collect();
        (url, tags)
    })
}


use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::io::prelude::*;

/*
 * Feeds file: one feed per line in the format:
 * <url> <comma-delimited tags>
 */
pub fn load_feeds<P>(path: P) -> impl Iterator<Item=(String, Vec<String>)> where P: AsRef<Path> {
    let file = File::open(path).unwrap();
    BufReader::new(file).lines().filter_map(Result::ok).map(|line| {
        let mut split = line.splitn(2, ' ');
        let url = split.next().unwrap().to_string();
        let tags = split.next().unwrap_or("").split(",").map(|s| s.to_string()).collect();
        (url, tags)
    })
}


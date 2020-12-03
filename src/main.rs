mod db;
mod ui;
mod sync;
mod conf;
mod events;

use webbrowser;
use self::sync::update;
use self::db::{Database, Item};
use std::{thread, time};
use self::conf::load_feeds;
use self::ui::{StatefulList, StatefulTable};
use self::events::{Events, Event};

use regex::{Regex, RegexBuilder};
use std::io;
use termion::raw::IntoRawMode;
use termion::event::Key;
use termion::input::TermRead;

use std::{error::Error};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    // widgets::{Block, Borders, List, ListItem},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Wrap},
    Terminal,
};

use chrono::{DateTime, TimeZone, NaiveDateTime, Utc, Local};

enum InputMode {
    Normal,
    Search,
}

struct App {
    items: StatefulList<Item>
}

impl App {
    fn new() -> App {
        App {
            items: StatefulList::with_items(vec![])
        }
    }
}



fn mark_selected_read(db: &Database, items: &mut Vec<Item>, table: &StatefulTable) {
    match table.state.selected() {
        Some(i) => {
            items[i].read = true;
            db.set_item_read(&items[i], true);
        },
        None => {}
    }
}

fn mark_selected_unread(db: &Database, items: &mut Vec<Item>, table: &StatefulTable) {
    match table.state.selected() {
        Some(i) => {
            items[i].read = false;
            db.set_item_read(&items[i], false);
        },
        None => {}
    }
}

fn split_keep<'a>(r: &Regex, text: &'a str) -> Vec<(&'a str, bool)> {
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

fn main() -> Result<(), Box<dyn Error>> {
    let db_path = "data/rsrss.db";
    let feeds_path = "data/feeds.txt";
    let update_interval = 10 * 60 * 1000; // ms
    // let handle = thread::spawn(move || {
    //     let update_duration = time::Duration::from_millis(update_interval);
    //     loop {
    //         println!("updating...");
    //         let db = Database::new(&db_path);
    //         for (feed_url, _tags) in load_feeds(&feeds_path) {
    //             println!("{:?}", feed_url);
    //             update(&feed_url, &db).unwrap();
    //         }
    //         println!("done updating");
    //         thread::sleep(update_duration);
    //     }
    // });
    // handle.join().unwrap();

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    // terminal.hide_cursor()?;

    let mut table = StatefulTable::new();
    let mut fullscreen_preview = false;

    let mut input_mode = InputMode::Normal;

    let mut search_results: Vec<usize> = Vec::new();

    let mut events = Events::new();
    let mut app = App::new();

    let db = Database::new(&db_path);
    for (feed_url, _tags) in load_feeds(&feeds_path) {
        // println!("{:?}", feed_url);
        update(&feed_url, &db).unwrap();
    }

    // TODO why do i need both flat map and flatten?
    let mut items: Vec<Item> = load_feeds(&feeds_path).flat_map(|(feed_url, _tags)| {
        // println!("{:?}", db.get_channel_items(&feed_url));
        db.get_channel_items(&feed_url).ok()
    }).flatten().collect();

    table.set_items(items.iter().map(|i| {
        let pub_date = match i.published_at {
            Some(ts) => Local.timestamp(ts, 0).format("%m/%d/%y %H:%M").to_string(),
            None => "<no pub date>".to_string()
        };

        vec![
            i.title.as_deref().unwrap_or("<no title>").to_string(),
            pub_date,
            i.channel.clone(),
        ]
    }).collect());

    // println!("{:?}", items.len());
    // println!("{:?}", table.items.len());

    terminal.clear()?;
    let mut scroll: u16 = 0;
    let mut search_input = String::new();
    let mut search_query = String::new();
    loop {
        terminal.draw(|f| {
            let (msg, style) = match input_mode {
                InputMode::Normal => (
                    vec![
                        Span::raw(format!("[{} unread] ", items.iter().filter(|i| !i.read).fold(0, |c, _| c + 1))),
                        Span::raw("["),
                        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" quit]"),
                        Span::raw(" ["),
                        Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" search]"),
                        Span::raw(" ["),
                        Span::styled("f", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" fullscreen]"),
                    ],
                    Style::default().add_modifier(Modifier::RAPID_BLINK),
                ),
                InputMode::Search => (
                    vec![
                        Span::raw("/"),
                        Span::styled(&search_input, Style::default().add_modifier(Modifier::BOLD)),
                    ],
                    Style::default(),
                ),
            };
            let mut text = Text::from(Spans::from(msg));
            text.patch_style(style);
            let status_bar = Paragraph::new(text).style(Style::default().bg(Color::DarkGray));

            let preview = match table.state.selected() {
                Some(i) =>  {
                    // let size = f.size();
                    // let s = "Veeeeeeeeeeeeeeeery    loooooooooooooooooong   striiiiiiiiiiiiiiiiiiiiiiiiiing.   ";
                    // let mut long_line = s.repeat(usize::from(size.width) / s.len() + 4);
                    // long_line.push('\n');
                    let item = &items[i];
                    let pub_date = match item.published_at {
                        Some(ts) => Local.timestamp(ts, 0).format("%B %d, %Y %H:%M").to_string(),
                        None => "<no pub date>".to_string()
                    };

                    let mut text = vec![
                        // Must be a better way
                        Spans::from(
                            Span::styled(item.title.as_deref().unwrap_or("<no title>"), Style::default().fg(Color::Yellow))),
                        Spans::from(pub_date),
                        Spans::from(item.channel.clone()),
                        Spans::from("\n"),
                        // Spans::from("This is a line "),
                        // Spans::from(Span::styled("This is a line   ", Style::default().fg(Color::Red))),
                        // Spans::from(Span::styled("This is a line", Style::default().bg(Color::Blue))),
                        // Spans::from(Span::styled(
                        //     "This is a longer line",
                        //     Style::default().add_modifier(Modifier::CROSSED_OUT),
                        // )),
                        // Spans::from(Span::styled(&long_line, Style::default().bg(Color::Green))),
                        // Spans::from(Span::styled(
                        //     "This is a line",
                        //     Style::default().fg(Color::Green).add_modifier(Modifier::ITALIC),
                        // )),
                    ];

                    for line in item.description.as_deref().unwrap_or("<no description>").split('\n') {
                        text.push(Spans::from(line));
                    }

                    Paragraph::new(text.clone())
                        .style(Style::default())//.bg(Color::White).fg(Color::Black))
                        .block(Block::default())
                            // .style(Style::default().bg(Color::White).fg(Color::Black)))
                        .alignment(Alignment::Left)
                        .wrap(Wrap { trim: true })
                        .scroll((scroll, 0))
                }
                None => Paragraph::new("No item selected.")
            };

            if (fullscreen_preview) {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                         Constraint::Min(1),
                         Constraint::Length(1),
                    ].as_ref())
                    .split(f.size());

                f.render_widget(preview, chunks[0]);
                f.render_widget(status_bar, chunks[1]);
            } else {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                         Constraint::Min(1),
                         Constraint::Percentage(50),
                         Constraint::Length(1),
                    ].as_ref())
                    .split(f.size());

                let selected_style = Style::default().add_modifier(Modifier::REVERSED);
                let normal_style = Style::default().bg(Color::White);
                let header_cells = ["Title", "Published"]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
                let header = Row::new(header_cells)
                    .style(normal_style)
                    .height(1);

                let reg = match input_mode {
                    InputMode::Normal => format!(r"({})", &search_query),
                    InputMode::Search => format!(r"({})", &search_input)
                };
                let separator = RegexBuilder::new(&reg).case_insensitive(true).build().expect("Invalid regex");

                let rows = table.items.iter().enumerate().map(|(i, item)| {
                    let height = item
                        .iter()
                        .map(|content| content.chars().filter(|c| *c == '\n').count())
                        .max()
                        .unwrap_or(1)
                        + 1;
                    let cells = item.iter().map(|c| {
                        let parts = split_keep(&separator, c);
                        let spans: Vec<Span> = parts.iter().map(|(text, is_match)| {
                            if *is_match {
                                Span::styled(*text, Style::default().fg(Color::Yellow))
                            } else {
                                Span::raw(*text)
                            }
                        }).collect();
                        Cell::from(Spans::from(spans))

                        // if search_results.contains(&i) {
                        //     let parts = split_keep(&separator, c);
                        //     let spans: Vec<Span> = parts.iter().map(|(text, is_match)| {
                        //         if *is_match {
                        //             Span::styled(*text, Style::default().fg(Color::Yellow))
                        //         } else {
                        //             Span::raw(*text)
                        //         }
                        //     }).collect();
                        //     Cell::from(Spans::from(spans))
                        // } else {
                        //     Cell::from(Spans::from(c.clone()))
                        // }
                    });
                    let style = if items[i].read {
                        Style::default().fg(Color::Rgb(100,100,100))
                    } else {
                        Style::default()
                    };
                    Row::new(cells).height(height as u16).style(style)
                });
                let t = Table::new(rows)
                    .header(header)
                    .block(Block::default().borders(Borders::BOTTOM))
                    .highlight_style(selected_style)
                    .widths(&[
                        Constraint::Percentage(50),
                        Constraint::Length(30),
                        Constraint::Max(10),
                    ]);
                f.render_stateful_widget(t, chunks[0], &mut table.state);

                f.render_widget(preview, chunks[1]);
                f.render_widget(status_bar, chunks[2]);

                // let items: Vec<ListItem> = app
                //     .items
                //     .items
                //     .iter()
                //     .map(|i| {
                //         let mut lines = vec![
                //         // let mut lines = vec![Spans::from(i.0)];
                //         // for _ in 0..i.1 {
                //         //     lines.push(Spans::from(Span::styled(
                //         //         "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                //         //         Style::default().add_modifier(Modifier::ITALIC),
                //         //     )));
                //         // }
                //         ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
                //     })
                //     .collect();
                // let items = List::new(items)
                //     .block(Block::default())
                //     .highlight_style(
                //         Style::default()
                //             .bg(Color::LightGreen)
                //             .add_modifier(Modifier::BOLD),
                //     )
                //     .highlight_symbol(">> ");
                // f.render_stateful_widget(items, chunks[0], &mut app.items.state);
            }
        })?;

        match events.next()? {
            Event::Input(input) => match input_mode {
                InputMode::Normal => match input {
                    Key::Char('q') => {
                        break;
                    }
                    Key::Left => {
                        // app.items.unselect();
                    }
                    Key::Char('u') => {
                        mark_selected_unread(&db, &mut items, &table);
                    }
                    // Key::Down => {
                    Key::Char('j') => {
                        // app.items.next();
                        table.next();
                        mark_selected_read(&db, &mut items, &table);
                        scroll = 0; // reset paragraph scroll
                    }
                    Key::Char('\n') => {
                        match table.state.selected() {
                            Some(i) => {
                                match &items[i].url {
                                    Some(url) => {
                                        webbrowser::open(&url);
                                    },
                                    None => {}
                                }
                            },
                            None => {}
                        };
                        // if webbrowser::open("http://github.com").is_ok() {
                    }

                    Key::Char('n') => {
                        if search_results.len() > 0 {
                            match table.state.selected() {
                                Some(i) => {
                                    if i >= *search_results.last().unwrap() {
                                        table.state.select(Some(search_results[0]));
                                    } else {
                                        for si in &search_results {
                                            if *si > i {
                                                table.state.select(Some(*si));
                                                break;
                                            }
                                        }
                                    }
                                },
                                None => {
                                    table.state.select(Some(search_results[0]));
                                }
                            }
                        }
                    }
                    Key::Ctrl('n') => {
                        if search_results.len() > 0 {
                            match table.state.selected() {
                                Some(i) => {
                                    if i <= search_results[0] {
                                        let last = search_results.last().unwrap();
                                        table.state.select(Some(*last));
                                    } else {
                                        for si in search_results.iter().rev() {
                                            if *si < i {
                                                table.state.select(Some(*si));
                                                break;
                                            }
                                        }
                                    }
                                },
                                None => {
                                    table.state.select(Some(search_results[0]));
                                }
                            }
                        }
                    }

                    // Key::Up => {
                    Key::Char('k') => {
                        // app.items.previous();
                        table.previous();
                        mark_selected_read(&db, &mut items, &table);
                        scroll = 0; // reset paragraph scroll
                    }
                    // TODO No idea this keypress doesn't register
                    // https://docs.rs/termion/1.5.5/termion/event/enum.Key.html
                    // docs say that some keys can't be modified by ctrl, but ctrl+j works elsewhere
                    // Key::Ctrl('j') => {
                    Key::Ctrl('m') => {
                        table.jump_forward(5);
                        mark_selected_read(&db, &mut items, &table);
                        scroll = 0; // reset paragraph scroll
                    }
                    Key::Ctrl('k') => {
                        table.jump_backward(5);
                        mark_selected_read(&db, &mut items, &table);
                        scroll = 0; // reset paragraph scroll
                    }
                    Key::Char('J') => {
                        scroll += 1;
                    }
                    Key::Char('K') => {
                        if (scroll > 0) {
                            scroll -= 1;
                        }
                    }
                    Key::Char('f') => {
                        fullscreen_preview = !fullscreen_preview;
                    }
                    Key::Char('/') => {
                        input_mode = InputMode::Search;
                        events.disable_exit_key();
                    }
                    _ => {}
                },
                InputMode::Search => match input {
                    Key::Char('\n') => {
                        search_query = search_input.drain(..).collect();

                        let reg = format!(r"({})", &search_query);
                        let regex = RegexBuilder::new(&reg).case_insensitive(true).build().expect("Invalid regex");
                        search_results = items.iter().enumerate().filter(|(i, item)| {
                            match &item.title {
                                Some(title) => regex.is_match(title),
                                None => false
                            }
                        }).map(|i| i.0).collect();
                        input_mode = InputMode::Normal;
                    }
                    Key::Char(c) => {
                        search_input.push(c);
                    }
                    Key::Backspace => {
                        search_input.pop();
                    }
                    Key::Esc => {
                        search_input.clear();
                        input_mode = InputMode::Normal;
                        events.enable_exit_key();
                    }
                    _ => {}
                }
            },
            Event::Update => {
                // TODO re-render
                // app.advance();
            }
        }
    }
    Ok(())
}

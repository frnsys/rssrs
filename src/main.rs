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

use std::io;
use termion::raw::IntoRawMode;
use termion::event::Key;
use termion::input::TermRead;

use std::{error::Error};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    // widgets::{Block, Borders, List, ListItem},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Wrap},
    Terminal,
};

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

    let events = Events::new();
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
        // This seems unnecessarily messy
        vec![
            i.title.as_deref().unwrap_or("<no title>").to_string(),
            i.published_at.as_deref().unwrap_or("<no pub date>").to_string(),
            // i.channel.clone(),
            i.channel.clone(),
        ]
    }).collect());

    // println!("{:?}", items.len());
    // println!("{:?}", table.items.len());

    terminal.clear()?;
    let mut scroll: u16 = 0;
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let selected_style = Style::default().add_modifier(Modifier::REVERSED);
            let normal_style = Style::default().bg(Color::White);
            let header_cells = ["Title", "Published"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
            let header = Row::new(header_cells)
                .style(normal_style)
                .height(1);

            let rows = table.items.iter().enumerate().map(|(i, item)| {
                let height = item
                    .iter()
                    .map(|content| content.chars().filter(|c| *c == '\n').count())
                    .max()
                    .unwrap_or(1)
                    + 1;
                let cells = item.iter().map(|c| Cell::from(c.clone()));
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

            match table.state.selected() {
                Some(i) =>  {
                    // let size = f.size();
                    // let s = "Veeeeeeeeeeeeeeeery    loooooooooooooooooong   striiiiiiiiiiiiiiiiiiiiiiiiiing.   ";
                    // let mut long_line = s.repeat(usize::from(size.width) / s.len() + 4);
                    // long_line.push('\n');
                    let item = &items[i];
                    let mut text = vec![
                        // Must be a better way
                        Spans::from(
                            Span::styled(item.title.as_deref().unwrap_or("<no title>"), Style::default().fg(Color::Yellow))),
                        Spans::from(item.published_at.as_deref().unwrap_or("<no publish date>")),
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

                    let paragraph = Paragraph::new(text.clone())
                        .style(Style::default())//.bg(Color::White).fg(Color::Black))
                        .block(Block::default())
                            // .style(Style::default().bg(Color::White).fg(Color::Black)))
                        .alignment(Alignment::Left)
                        .wrap(Wrap { trim: true })
                        .scroll((scroll, 0));
                    f.render_widget(paragraph, chunks[1]);
                }
                None => {}
            }

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
        })?;

        match events.next()? {
            Event::Input(input) => match input {
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
                Key::Ctrl('n') => {
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
                _ => {}
            },
            Event::Update => {
                // TODO re-render
                // app.advance();
            }
        }
    }
    Ok(())
}

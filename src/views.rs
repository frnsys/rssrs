use super::app::{App, InputMode, Status};
use super::util::split_keep;
use chrono::{TimeZone, Local};
use tui::{
    terminal::Frame,
    backend::Backend,
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    layout::{Constraint, Direction, Layout, Alignment},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Wrap},
};


pub fn render_browser<B>(app: &mut App, frame: &mut Frame<B>) where B: Backend {
    // Status bar
    let update_str = match app.status {
        Status::Updating => "Updating...",
        _ => ""
    };
    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw(update_str),
                Span::raw(format!("[{}{}{}{}]", match app.filter.read {
                    Some(b) => if b {
                        "R"
                    } else {
                        "Я"
                    },
                    None => ""
                }, match app.filter.starred {
                    Some(b) => if b {
                        "S"
                    } else {
                        "Ƨ"
                    },
                    None => ""
                }, if app.filter.tags.len() == 0 {
                    ""
                } else {
                    "T"
                }, if app.filter.feeds.len() == 0 {
                    ""
                } else {
                    "C"
                })),
                Span::raw(format!("[{} unread] ", app.items.iter().filter(|i| !i.read).fold(0, |c, _| c + 1))),
            ],
            Style::default(),
        ),
        InputMode::Search => (
            vec![
                Span::raw("/"),
                Span::styled(&app.search_input_raw, Style::default().add_modifier(Modifier::BOLD)),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let status_bar = Paragraph::new(text).style(Style::default().bg(Color::DarkGray));

    // Reader
    let reader = match app.table.state.selected() {
        Some(i) =>  {
            let item = &app.items[i];
            let pub_date = match item.published_at {
                Some(ts) => Local.timestamp(ts, 0).format("%B %d, %Y %H:%M").to_string(),
                None => "<no pub date>".to_string()
            };

            let mut text = vec![
                Spans::from(
                    Span::styled(item.title.as_deref().unwrap_or("<no title>"), Style::default().fg(Color::Yellow))),
                Spans::from(format!("{} ({})", app.feeds[&item.feed].title.clone(), item.feed.clone())),
                Spans::from(item.url.as_deref().unwrap_or("<no url>")),
                Spans::from(pub_date),
                Spans::from("\n"),
            ];

            for line in item.description.as_deref().unwrap_or("<no description>").split('\n') {
                text.push(Spans::from(line));
            }

            Paragraph::new(text.clone())
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true })
                .scroll((app.reader_scroll, 0))
        }
        None => Paragraph::new("No item selected.")
    };


    if app.focus_reader {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                 Constraint::Min(1),
                 Constraint::Length(1),
            ].as_ref())
            .split(frame.size());

        frame.render_widget(reader, chunks[0]);
        frame.render_widget(status_bar, chunks[1]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                 Constraint::Min(1),
                 Constraint::Percentage(50),
                 Constraint::Length(1),
            ].as_ref())
            .split(frame.size());

        // Item list
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_cells = ["Title", "Published"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1);

        let regex = match app.input_mode {
            InputMode::Normal => &app.search_query,
            InputMode::Search => &app.search_input
        };

        let rows = app.table.items.iter().enumerate().map(|(i, item)| {
            let height = item
                .iter()
                .map(|content| content.chars().filter(|c| *c == '\n').count())
                .max()
                .unwrap_or(1)
                + 1;
            let cells = item.iter().map(|c| {
                let spans: Vec<Span> = match regex {
                    Some(re) => {
                        let parts = split_keep(re, c);
                        parts.iter().map(|(text, is_match)| {
                            if *is_match {
                                Span::styled(*text, Style::default().fg(Color::Yellow))
                            } else {
                                Span::raw(*text)
                            }
                        }).collect()
                    },
                    None => vec![Span::raw(c)]
                };
                Cell::from(Spans::from(spans))
            });


            // Color according to read and/or marked status
            let mut style = Style::default();
            if app.items[i].read {
                style = style.fg(Color::Rgb(100,100,100));
            }
            if app.marked.contains(&i) {
                style = style.bg(Color::DarkGray);
            }
            if app.items[i].starred {
                style = style.add_modifier(Modifier::BOLD).fg(Color::Yellow);
            }

            Row::new(cells).height(height as u16).style(style)
        });
        let item_list = Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::BOTTOM))
            .highlight_style(selected_style)
            .widths(&[
                Constraint::Percentage(70),
                Constraint::Length(16),
            ]);

        frame.render_stateful_widget(item_list, chunks[0], &mut app.table.state);
        frame.render_widget(reader, chunks[1]);
        frame.render_widget(status_bar, chunks[2]);
    }
}

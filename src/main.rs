mod db;
mod app;
mod util;
mod sync;
mod conf;
mod views;
mod events;

use std::{io, error::Error};
use self::app::{App, Status, InputMode};
use self::conf::Config;
use self::events::{Events, Event};
use termion::raw::IntoRawMode;
use termion::event::Key;
use tui::{
    backend::TermionBackend,
    Terminal,
};


fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::default();
    let mut app = App::new(&config.db_path, &config.feeds_path);
    app.load_items();

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut events = Events::with_config(config.clone());

    terminal.clear()?;
    loop {
        terminal.draw(|mut f| {
            views::render_browser(&mut app, &mut f);
        })?;

        match events.next()? {
            Event::Input(input) => match app.input_mode {
                InputMode::Normal => match input {
                    Key::Char('q') => break,
                    Key::Char('u') => app.mark_selected_unread(),
                    Key::Char('j') => app.scroll_items_down(),
                    Key::Char('k') => app.scroll_items_up(),
                    Key::Ctrl('d') => app.page_items_down(),
                    Key::Ctrl('u') => app.page_items_up(),
                    Key::Char('o') => app.open_selected(),
                    Key::Char('J') => app.scroll_reader_down(),
                    Key::Char('K') => app.scroll_reader_up(),
                    Key::Char('n') => app.jump_to_next_result(),
                    Key::Char('N') => app.jump_to_prev_result(),
                    Key::Char('f') => app.toggle_focus_reader(),
                    Key::Char('/') => {
                        app.start_search();
                        events.disable_exit_key();
                    }
                    _ => {}
                },
                InputMode::Search => match input {
                    Key::Char('\n') => {
                        let search_query = app.search_input_raw.drain(..).collect();
                        let search_query = app.build_query(&search_query);
                        app.execute_search(&search_query);
                        app.search_query = Some(search_query);
                        app.end_search();
                    }
                    Key::Char(c) => {
                        app.search_input_raw.push(c);
                        app.search_input = Some(app.build_query(&app.search_input_raw));
                    }
                    Key::Backspace => {
                        app.search_input_raw.pop();
                        app.search_input = Some(app.build_query(&app.search_input_raw));
                    }
                    Key::Esc => {
                        app.end_search();
                        events.enable_exit_key();
                    }
                    _ => {}
                }
            },
            Event::Updating => {
                app.status = Status::Updating;
            }
            Event::Updated => {
                app.status = Status::Idle;
                app.load_items();
            }
        }
    }

    terminal.clear()?;
    Ok(())
}

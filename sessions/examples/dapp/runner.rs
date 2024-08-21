use crate::app;
use crate::event_reader::EventReader;
use crate::ui::UI;
use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, ExecutableCommand};
use ratatui::prelude::*;
use std::io::stdout;
use std::sync::mpsc;

pub struct Runner {}

impl Runner {
    pub fn run(self) -> anyhow::Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;
        let app = app::App::new();
        let mut ui = UI::new();
        let (tx_msg_in, rx_msg_in) = mpsc::channel::<app::Task>();
        let mut event_reader = EventReader::new(tx_msg_in.clone());
        event_reader.start();

        loop {
            terminal.draw(|frame| ui.draw(frame, &app))?;
            if event::poll(std::time::Duration::from_millis(16))? {
                if let event::Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }

        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }
}

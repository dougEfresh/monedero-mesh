use crate::app;
use crate::event_reader::EventReader;
use crate::msg::out::MsgOut;
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
        let mut app = app::App::new();
        let mut ui = UI::new();
        let (tx_msg_in, rx_msg_in) = mpsc::channel::<app::Task>();
        let mut event_reader = EventReader::new(tx_msg_in.clone());
        event_reader.start();

        tx_msg_in.send(app::Task::new(
            app::MsgIn::External(app::ExternalMsg::Refresh),
            None,
        ))?;
        let mut result = Ok(None);
        'outer: for task in rx_msg_in {
            match app.handle_task(task) {
                Ok(a) => {
                    app = a;
                    while let Some(msg) = app.msg_out.pop_front() {
                        use app::MsgOut::*;
                        match msg {
                            Refresh => {
                                terminal.draw(|f| ui.draw(f, &app))?;
                            }
                            ClearScreen => terminal.clear()?,
                            Debug(_) => {}
                            EnableMouse => {}
                            DisableMouse => {}
                            ToggleMouse => {}
                            StartFifo(_) => {}
                            StopFifo => {}
                            ToggleFifo(_) => {}
                            ScrollUp => {}
                            ScrollDown => {}
                            ScrollUpHalf => {}
                            ScrollDownHalf => {}
                            Quit => break 'outer,
                            Enqueue(task) => {
                                tx_msg_in.send(task)?;
                            }
                        }
                    }
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }

        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }
}

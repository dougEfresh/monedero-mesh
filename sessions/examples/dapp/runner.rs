use crate::app;
use crate::event_reader::EventReader;
use crate::ui::UI;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, execute, ExecutableCommand};
use ratatui::prelude::*;
use std::io::stdout;
use std::panic::{set_hook, take_hook};
use std::sync::mpsc;

pub struct Runner {}

pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

pub fn restore_tui() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

impl Runner {
    pub fn run(self) -> anyhow::Result<()> {
        init_panic_hook();
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
        let mut result = Ok(());
        'outer: for task in rx_msg_in {
            match app.handle_task(task) {
                Ok(a) => {
                    app = a;
                    while let Some(msg) = app.msg_out.pop_front() {
                        use app::MsgOut::*;
                        tracing::debug!("handling message: {:?}", msg);
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
                            Quit => {
                                tracing::info!("quiting");
                                break 'outer;
                            }
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

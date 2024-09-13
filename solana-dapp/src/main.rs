use std::panic;

use ratatui::crossterm::{
    cursor::Show,
    event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use monedero_mesh::{Dapp, Metadata, ProjectId, WalletConnectBuilder};

use crate::log::initialize_logging;
use crate::runner::Runner;

pub mod app;
pub mod event;
pub mod handler;
pub mod tui;
pub mod ui;
mod log;
mod message;
mod runner;
mod workers;
pub use message::Msg;
use crate::ui::Ui;

macro_rules! enable_raw_mode {
    () => {
        enable_raw_mode().expect("failed to enable raw mode");
        execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        )
        .expect("failed to enable raw mode");
    };
}

macro_rules! disable_raw_mode {
    () => {
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableFocusChange,
            Show
        )
        .expect("failed to restore terminal");
        disable_raw_mode().expect("failed to disable raw mode");
    };
}

fn set_signal_handler() {
    ctrlc::set_handler(|| {
        disable_raw_mode!();

        std::process::exit(0);
    })
      .expect("Error setting Ctrl-C handler")
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_logging()?;
    /*
    set_signal_handler();
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));
     */

    let project = ProjectId::from("1760736b8b49aeb707b1a80099e51e58");
    let auth = monedero_mesh::auth_token("https://github.com/dougEfresh");
    let mgr = WalletConnectBuilder::new(project, auth).build().await?;
    let dapp = Dapp::new(mgr, Metadata{
        name: env!("CARGO_CRATE_NAME").to_string(),
        description: "solana-dapp".to_string(),
        url: "https://github.com/dougeEfresh/monedero-mesh".to_string(),
        icons: vec![],
        verify_url: None,
        redirect: None,
    }).await?;

    Ui::init()?.run();
    Ok(())
    // Create an application.
    //let mut app = App::new();

    //enable_raw_mode!();
    //let result = Runner::run();
    //disable_raw_mode!();
    // Initialize the terminal user interface.
    /*
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
     */
}

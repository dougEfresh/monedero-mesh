use crate::log::initialize_logging;

pub mod app;
mod config;
pub mod handler;
mod log;
mod message;
mod session_poll;
pub mod ui;

use crate::config::AppConfig;
use crate::session_poll::SessionPoll;
use crate::ui::Ui;
pub use message::Msg;
use monedero_solana::monedero_mesh::{auth_token, Dapp, Metadata, ProjectId, WalletConnectBuilder};
pub use session_poll::DappContext;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_logging()?;
    /*

    */
    let cfg = AppConfig::default();
    let poller = SessionPoll::init(cfg.clone()).await?;
    Ui::init(poller)?.run();
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

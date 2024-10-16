use monedero_solana::monedero_mesh::Pairing;
use tokio::sync::mpsc::error::TryRecvError;
use widgetui::{Chunks, Events, ResMut, State, WidgetFrame, WidgetResult};
use widgetui::widget::WidgetError;
use ratatui::widgets::Paragraph;
use crate::{CustomChunk, SettlementState};

pub struct PairingState {
    pub pairing: Pairing,
    pub settlement: Option<SettlementState>,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<SettlementState>
}

impl PairingState {
    pub fn finalized(&mut self) -> Option<SettlementState> {
        if self.settlement.is_some() {
            return self.settlement.clone()
        }

        self.settlement = match self.rx.try_recv() {
            Ok(_) => Some(SettlementState::Settled),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(SettlementState::Error("disconnected".to_string())),
        };
        self.settlement.clone()
    }
}

impl State for PairingState {}

pub fn pair_widget(
    mut frame: ResMut<WidgetFrame>,
    mut pair_state: ResMut<PairingState>,
    mut events: ResMut<Events>,
    mut chunks: ResMut<Chunks>, ) -> WidgetResult {

    chunks.register_chunk::<CustomChunk>(frame.size());
    let chunk = chunks.get_chunk::<CustomChunk>()?;

    if events.key(crossterm::event::KeyCode::Char('q')) {
        events.register_exit();
        let err: WidgetError = WidgetError::Misc(anyhow::format_err!("quit"));
        return Err(err);
    }

    match pair_state.finalized() {
        None => {}
        Some(SettlementState::Settled) => {
            events.register_exit();
            return Ok(());
        },
        Some(SettlementState::Error(e)) => {
            frame.render_widget(
                Paragraph::new(format!("Error {}", pair_state.pairing)),
                chunk,
            );
            return Ok(())
        }
    }
    frame.render_widget(
        Paragraph::new(format!("{}", pair_state.pairing)),
        chunk,
    );
    Ok(())
}

use crate::ui::{draw_area_in, Id, Ui};
use tuirealm::tui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::tui::widgets::Clear;

impl Ui {
    pub(super) fn view(&mut self) {
        let _ = self.terminal.raw_mut().draw(|f| {
            /*
            let main_chunks = Layout::default()
              .direction(Direction::Vertical)
              .margin(1)
              .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
              .split(f.size());
             */
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.size());

            self.application.view(&Id::NavBar, f, chunks[0]);
            self.application.view(&Id::Home, f, chunks[1]);
            self.application.view(&Id::Legend, f, chunks[2]);

            if self.application.mounted(&Id::QuitPopup) {
                let popup = draw_area_in(f.size(), 30, 10);
                f.render_widget(Clear, popup);
                self.application.view(&Id::QuitPopup, f, popup);
            } else if self.application.mounted(&Id::ErrorPopup) {
                let popup = draw_area_in(f.size(), 50, 15);
                f.render_widget(Clear, popup);
                self.application.view(&Id::ErrorPopup, f, popup);
            } else if self.application.mounted(&Id::Pairing) {
                let popup = draw_area_in(f.size(), 60, 35);
                f.render_widget(Clear, popup);
                self.application.view(&Id::Pairing, f, popup);
            }
        });
    }
}

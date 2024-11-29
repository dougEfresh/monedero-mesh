use {
    crate::app,
    ratatui::{
        prelude::*,
        widgets::{List, ListItem, Paragraph},
        Frame,
    },
};
mod layout;
pub use layout::*;

pub struct UI {
    pub screen_size: Rect,
    pub scrolltop: usize,
}

impl UI {
    pub fn new() -> Self {
        Self {
            screen_size: Default::default(),
            scrolltop: 0,
        }
    }

    fn draw_table(&mut self, f: &mut Frame, layout_size: Rect, app: &app::App) {
        let table = Paragraph::new("Hello Ratatui! (press 'q' to quit)")
            .white()
            .on_blue();
        f.render_widget(table, layout_size);
    }

    fn draw_selection(&mut self, f: &mut Frame, layout_size: Rect, app: &app::App) {
        let table = Paragraph::new("Hello Ratatui! (press 'q' to quit)")
            .black()
            .on_white();
        f.render_widget(table, layout_size);
    }

    fn draw_help_menu(&mut self, f: &mut Frame, layout_size: Rect, app: &app::App) {
        let table = Paragraph::new("Hello Ratatui! (press 'q' to quit)")
            .green()
            .on_red();
        f.render_widget(table, layout_size);
    }

    fn draw_logs(&mut self, f: &mut Frame, layout_size: Rect, app: &app::App) {
        let items = vec![ListItem::new("log")];
        let logs_list = List::new(items);
        f.render_widget(logs_list, layout_size);
    }

    fn draw_layout(&mut self, layout: AppLayout, f: &mut Frame, layout_size: Rect, app: &app::App) {
        match layout {
            AppLayout::Table => self.draw_table(f, layout_size, app),
            AppLayout::SortAndFilter => {}
            AppLayout::HelpMenu => self.draw_help_menu(f, layout_size, app),
            AppLayout::Selection => self.draw_selection(f, layout_size, app),
            AppLayout::InputAndLogs => {
                // if app.input.buffer.is_some() {
                // self.draw_input_buffer(f, layout_size, app);
                //} else {
                self.draw_logs(f, layout_size, app);
                //};
            }
            AppLayout::Dynamic(ref func) => {}
            AppLayout::Horizontal { config, splits } => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        config
                            .constraints
                            .clone()
                            .unwrap_or_default()
                            .iter()
                            .map(|c| c.to_tui(self.screen_size, layout_size))
                            .collect::<Vec<Constraint>>(),
                    )
                    .horizontal_margin(
                        config
                            .horizontal_margin
                            .or(config.margin)
                            .unwrap_or_default(),
                    )
                    .vertical_margin(config.vertical_margin.or(config.margin).unwrap_or_default())
                    .split(layout_size);

                splits
                    .into_iter()
                    .zip(chunks.iter())
                    .for_each(|(split, chunk)| self.draw_layout(split, f, *chunk, app));
            }
            AppLayout::Vertical { config, splits } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        config
                            .constraints
                            .clone()
                            .unwrap_or_default()
                            .iter()
                            .map(|c| c.to_tui(self.screen_size, layout_size))
                            .collect::<Vec<Constraint>>(),
                    )
                    .horizontal_margin(
                        config
                            .horizontal_margin
                            .or(config.margin)
                            .unwrap_or_default(),
                    )
                    .vertical_margin(config.vertical_margin.or(config.margin).unwrap_or_default())
                    .split(layout_size);

                splits
                    .into_iter()
                    .zip(chunks.iter())
                    .for_each(|(split, chunk)| self.draw_layout(split, f, *chunk, app));
            }
            AppLayout::Nothing => {}
        }
    }

    pub fn draw(&mut self, f: &mut Frame, app: &app::App) {
        self.screen_size = f.area();
        let layout = app.mode.layout.clone();
        self.draw_layout(layout, f, self.screen_size, app);
    }
}

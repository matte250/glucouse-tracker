use eframe::App;
use eframe::egui;
use eframe::egui::CentralPanel;

use crate::db::Database;
use crate::ui::entry_form::{EntryFormState, show_entry_form};
use crate::ui::graph::{GraphState, refresh_graph, show_graph};
use crate::ui::readings_list::{ReadingsListState, refresh_readings, show_readings_list};

pub struct GlucoseTrackerApp {
    db: Database,
    entry_form: EntryFormState,
    readings_list: ReadingsListState,
    graph: GraphState,
    needs_refresh: bool,
}

impl GlucoseTrackerApp {
    pub fn new(db: Database) -> Self {
        let mut app = Self {
            db,
            entry_form: EntryFormState::default(),
            readings_list: ReadingsListState::default(),
            graph: GraphState::default(),
            needs_refresh: true,
        };
        refresh_readings(&mut app.readings_list, &app.db);
        refresh_graph(&mut app.graph, &app.db);
        app
    }
}

impl App for GlucoseTrackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.needs_refresh {
            refresh_readings(&mut self.readings_list, &self.db);
            refresh_graph(&mut self.graph, &self.db);
            self.needs_refresh = false;
        }

        egui::SidePanel::left("entry_panel")
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("entry_panel_scroll")
                    .show(ui, |ui| {
                        let mut changed = false;
                        show_entry_form(ui, &mut self.entry_form, &self.db, &mut changed);
                        if changed {
                            self.needs_refresh = true;
                        }
                    });
            });

        egui::SidePanel::right("graph_panel")
            .default_width(450.0)
            .show(ctx, |ui| {
                show_graph(ui, &mut self.graph, &self.db);
            });

        CentralPanel::default().show(ctx, |ui| {
            let mut changed = false;
            if let Some((date, time, value)) =
                show_readings_list(ui, &mut self.readings_list, &self.db, &mut changed)
            {
                self.entry_form.single_date = date;
                self.entry_form.single_time = time;
                self.entry_form.single_value = value;
            }
            if changed {
                self.needs_refresh = true;
            }
        });
    }
}

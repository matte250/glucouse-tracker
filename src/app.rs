use std::path::PathBuf;

use eframe::App;
use eframe::egui;
use eframe::egui::CentralPanel;

use glucose_tracker::db::Database;
use crate::ui::entry_form::{EntryFormState, show_entry_form};
use crate::ui::graph::{GraphState, refresh_graph, show_graph};
use crate::ui::readings_list::{ReadingsListState, refresh_readings, show_readings_list};

pub struct GlucoseTrackerApp {
    db: Database,
    entry_form: EntryFormState,
    readings_list: ReadingsListState,
    graph: GraphState,
    needs_refresh: bool,
    export_pending: Option<PathBuf>,
}

impl GlucoseTrackerApp {
    pub fn new(db: Database) -> Self {
        let mut app = Self {
            db,
            entry_form: EntryFormState::default(),
            readings_list: ReadingsListState::default(),
            graph: GraphState::default(),
            needs_refresh: true,
            export_pending: None,
        };
        refresh_readings(&mut app.readings_list, &app.db);
        refresh_graph(&mut app.graph, &app.db);
        app
    }
}

impl App for GlucoseTrackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle screenshot events
        if let Some(ref path) = self.export_pending.clone() {
            let events = ctx.input(|i| i.events.clone());
            for event in &events {
                if let egui::Event::Screenshot { image, .. } = event {
                    let cropped = if let Some(rect) = self.graph.graph_rect {
                        let ppp = ctx.pixels_per_point();
                        image.region(&rect, Some(ppp))
                    } else {
                        image.as_ref().clone()
                    };

                    if let Err(e) = glucose_tracker::export::export_pdf(path, &cropped, &self.graph.readings) {
                        eprintln!("Failed to save PDF: {e}");
                    }
                    self.export_pending = None;
                    break;
                }
            }
        }

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

        let _graph_response = egui::SidePanel::right("graph_panel")
            .default_width(450.0)
            .show(ctx, |ui| {
                show_graph(ui, &mut self.graph, &self.db);
            });
        // After rendering the graph, check if an export was requested
        if self.graph.export_requested {
            if let Some(path) = self.graph.export_path.take() {
                self.export_pending = Some(path);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            self.graph.export_requested = false;
        }

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

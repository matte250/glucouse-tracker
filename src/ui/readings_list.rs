use eframe::egui::{self, Ui};

use crate::db::Database;
use crate::models::GlucoseReading;

pub struct ReadingsListState {
    pub readings: Vec<GlucoseReading>,
    pub filter_text: String,
}

impl Default for ReadingsListState {
    fn default() -> Self {
        Self {
            readings: Vec::new(),
            filter_text: String::new(),
        }
    }
}

pub fn refresh_readings(state: &mut ReadingsListState, db: &Database) {
    state.readings = db.get_readings(None, None).unwrap_or_default();
}

pub fn show_readings_list(
    ui: &mut Ui,
    state: &mut ReadingsListState,
    db: &Database,
    changed: &mut bool,
) {
    ui.heading("Readings");
    ui.separator();

    ui.horizontal(|ui: &mut Ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.filter_text);
    });

    ui.separator();

    let filter = state.filter_text.trim().to_lowercase();
    let mut to_delete: Option<i64> = None;

    egui::ScrollArea::vertical()
        .id_salt("readings_list_scroll")
        .show(ui, |ui: &mut Ui| {
            egui::Grid::new("readings_table")
                .num_columns(4)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui: &mut Ui| {
                    ui.strong("Date");
                    ui.strong("Time");
                    ui.strong("Value (mmol/L)");
                    ui.strong("");
                    ui.end_row();

                    for reading in &state.readings {
                        let date_str = reading.recorded_at.format("%Y-%m-%d").to_string();
                        let time_str = reading.recorded_at.format("%H:%M").to_string();
                        let value_str = format!("{:.1}", reading.value);

                        if !filter.is_empty() {
                            let combined = format!("{date_str} {time_str} {value_str}").to_lowercase();
                            if !combined.contains(&filter) {
                                continue;
                            }
                        }

                        ui.label(&date_str);
                        ui.label(&time_str);
                        ui.label(&value_str);
                        if ui.small_button("Delete").clicked() {
                            to_delete = Some(reading.id);
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some(id) = to_delete {
        let _ = db.delete_reading(id);
        *changed = true;
    }
}

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::{self, Ui};
use egui_plot::{HLine, Line, Plot, PlotPoints};

use crate::db::Database;
use crate::models::GlucoseReading;

pub struct GraphState {
    pub from_date: String,
    pub to_date: String,
    pub readings: Vec<GlucoseReading>,
}

impl Default for GraphState {
    fn default() -> Self {
        let now = Local::now().naive_local();
        let from = now.date() - chrono::Duration::days(30);
        Self {
            from_date: from.format("%Y-%m-%d").to_string(),
            to_date: now.format("%Y-%m-%d").to_string(),
            readings: Vec::new(),
        }
    }
}

pub fn refresh_graph(state: &mut GraphState, db: &Database) {
    let from = NaiveDate::parse_from_str(&state.from_date, "%Y-%m-%d")
        .ok()
        .map(|d| NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
    let to = NaiveDate::parse_from_str(&state.to_date, "%Y-%m-%d")
        .ok()
        .map(|d| NaiveDateTime::new(d, NaiveTime::from_hms_opt(23, 59, 59).unwrap()));

    let mut readings = db.get_readings(from, to).unwrap_or_default();
    readings.sort_by_key(|r| r.recorded_at);
    state.readings = readings;
}

pub fn show_graph(ui: &mut Ui, state: &mut GraphState, db: &Database) {
    ui.heading("Glucose Graph");
    ui.separator();

    let mut range_changed = false;
    ui.horizontal(|ui: &mut Ui| {
        ui.label("From:");
        if ui.text_edit_singleline(&mut state.from_date).changed() {
            range_changed = true;
        }
        ui.label("To:");
        if ui.text_edit_singleline(&mut state.to_date).changed() {
            range_changed = true;
        }
        if ui.button("Refresh").clicked() {
            range_changed = true;
        }
    });

    if range_changed {
        refresh_graph(state, db);
    }

    if state.readings.is_empty() {
        ui.label("No readings in selected range.");
        return;
    }

    // Use the earliest reading as the reference epoch for the x-axis
    let epoch = state.readings.first().unwrap().recorded_at.and_utc().timestamp() as f64;

    let points: Vec<[f64; 2]> = state
        .readings
        .iter()
        .map(|r| {
            let x = r.recorded_at.and_utc().timestamp() as f64 - epoch;
            [x, r.value]
        })
        .collect();

    Plot::new("glucose_plot")
        .height(300.0)
        .x_axis_label("Time (seconds from first reading)")
        .y_axis_label("mmol/L")
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(PlotPoints::new(points)).name("Glucose"));

            // Threshold lines
            plot_ui.hline(
                HLine::new(4.0)
                    .name("Low (4.0)")
                    .color(egui::Color32::from_rgb(255, 165, 0))
                    .width(1.5),
            );
            plot_ui.hline(
                HLine::new(10.0)
                    .name("High (10.0)")
                    .color(egui::Color32::from_rgb(255, 60, 60))
                    .width(1.5),
            );
        });
}

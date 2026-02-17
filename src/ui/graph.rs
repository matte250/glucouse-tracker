use std::path::PathBuf;

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::{self, Ui};
use egui_plot::{GridInput, GridMark, HLine, Line, Plot, PlotPoints};

use crate::db::Database;
use crate::models::GlucoseReading;

pub struct GraphState {
    pub from_date: String,
    pub to_date: String,
    pub readings: Vec<GlucoseReading>,
    pub export_requested: bool,
    pub export_path: Option<PathBuf>,
    pub graph_rect: Option<egui::Rect>,
}

impl Default for GraphState {
    fn default() -> Self {
        let now = Local::now().naive_local();
        let from = now.date() - chrono::Duration::days(30);
        Self {
            from_date: from.format("%Y-%m-%d").to_string(),
            to_date: now.format("%Y-%m-%d").to_string(),
            readings: Vec::new(),
            export_requested: false,
            export_path: None,
            graph_rect: None,
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
        if ui.button("Export PDF").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("glucose_report.pdf")
                .save_file()
            {
                state.export_path = Some(path);
                state.export_requested = true;
            }
        }
    });

    if range_changed {
        refresh_graph(state, db);
    }

    if state.readings.is_empty() {
        ui.label("No readings in selected range.");
        return;
    }

    let points: Vec<[f64; 2]> = state
        .readings
        .iter()
        .map(|r| {
            let x = r.recorded_at.and_utc().timestamp() as f64;
            [x, r.value]
        })
        .collect();

    let scope = ui.scope(|ui| {
        Plot::new("glucose_plot")
            .height(300.0)
            .y_axis_label("mmol/L")
            .x_grid_spacer(|input: GridInput| {
                let day = 86400.0_f64;
                let hour = 3600.0_f64;
                let (min, max) = input.bounds;
                let mut marks = Vec::new();

                // Thickest lines at day boundaries, medium at 6h, thin at 1h
                let intervals = [day, hour * 6.0, hour];

                for &step in &intervals {
                    if step < input.base_step_size * 0.5 {
                        continue;
                    }
                    let first = (min / step).ceil() as i64;
                    let last = (max / step).floor() as i64;
                    for i in first..=last {
                        let value = i as f64 * step;
                        marks.push(GridMark {
                            value,
                            step_size: step,
                        });
                    }
                }
                marks
            })
            .x_axis_formatter(|mark: GridMark, _range: &std::ops::RangeInclusive<f64>| {
                let secs = mark.value as i64;
                match chrono::DateTime::from_timestamp(secs, 0) {
                    Some(dt) => dt
                        .with_timezone(&Local)
                        .format("%b %d %H:%M")
                        .to_string(),
                    None => String::new(),
                }
            })
            .label_formatter(|name, point| {
                let secs = point.x as i64;
                let time_str = match chrono::DateTime::from_timestamp(secs, 0) {
                    Some(dt) => dt
                        .with_timezone(&Local)
                        .format("%b %d %H:%M")
                        .to_string(),
                    None => format!("{}", point.x),
                };
                if name.is_empty() {
                    format!("{time_str}\n{:.1} mmol/L", point.y)
                } else {
                    format!("{name}\n{time_str}\n{:.1} mmol/L", point.y)
                }
            })
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
    });
    state.graph_rect = Some(scope.response.rect);

}

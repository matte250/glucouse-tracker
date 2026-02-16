use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::{self, Grid, Key, Ui};

use crate::db::Database;

pub struct EntryFormState {
    pub single_date: String,
    pub single_time: String,
    pub single_value: String,
    pub status_msg: String,
}

impl Default for EntryFormState {
    fn default() -> Self {
        let now = Local::now().naive_local();
        Self {
            single_date: now.format("%Y-%m-%d").to_string(),
            single_time: now.format("%H:%M").to_string(),
            single_value: String::new(),
            status_msg: String::new(),
        }
    }
}

fn parse_datetime(date: &str, time: &str) -> Option<NaiveDateTime> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let t = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    Some(NaiveDateTime::new(d, t))
}

pub fn show_entry_form(ui: &mut Ui, state: &mut EntryFormState, db: &Database, changed: &mut bool) {
    ui.heading("Add Reading");
    ui.separator();

    let enter_pressed = Grid::new("single_entry_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui: &mut Ui| {
            ui.label("Date:");
            let date_resp = ui.text_edit_singleline(&mut state.single_date);
            if date_resp.has_focus() {
                let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
                let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
                if up || down {
                    if let Ok(d) = NaiveDate::parse_from_str(&state.single_date, "%Y-%m-%d") {
                        let delta = if up { Duration::days(1) } else { Duration::days(-1) };
                        if let Some(new_d) = d.checked_add_signed(delta) {
                            state.single_date = new_d.format("%Y-%m-%d").to_string();
                        }
                    }
                }
            }
            ui.end_row();

            ui.label("");
            ui.label(egui::RichText::new("Up/Down: +/- 1 day").weak().small());
            ui.end_row();

            ui.label("Time:");
            let time_resp = ui.text_edit_singleline(&mut state.single_time);
            if time_resp.has_focus() {
                let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
                let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
                if up || down {
                    if let Some(dt) = parse_datetime(&state.single_date, &state.single_time) {
                        let delta = if up { Duration::hours(12) } else { Duration::hours(-12) };
                        let new_dt = dt + delta;
                        state.single_date = new_dt.format("%Y-%m-%d").to_string();
                        state.single_time = new_dt.format("%H:%M").to_string();
                    }
                }
            }
            ui.end_row();

            ui.label("");
            ui.label(egui::RichText::new("Up/Down: +/- 12h").weak().small());
            ui.end_row();

            ui.label("Value (mmol/L):");
            let value_resp = ui.text_edit_singleline(&mut state.single_value);
            let enter = value_resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            ui.end_row();

            enter
        }).inner;

    if ui.button("Add").clicked() || enter_pressed {
        if let Some(dt) = parse_datetime(&state.single_date, &state.single_time) {
            if let Ok(v) = state.single_value.trim().parse::<f64>() {
                if let Err(e) = db.insert_reading(v, dt) {
                    state.status_msg = format!("DB error: {e}");
                } else {
                    state.status_msg = format!("Added {v} mmol/L at {dt}");
                    *changed = true;
                }
            } else {
                state.status_msg = "Invalid glucose value".into();
            }
        } else {
            state.status_msg = "Invalid date/time (use YYYY-MM-DD and HH:MM)".into();
        }
    }

    if !state.status_msg.is_empty() {
        ui.separator();
        ui.label(&state.status_msg);
    }
}

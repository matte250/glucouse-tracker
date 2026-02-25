mod app;
mod ui;

use app::GlucoseTrackerApp;
use glucose_tracker::db::Database;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let db = Database::open("glucose_tracker.db").expect("Failed to open database");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Glucose Tracker",
        options,
        Box::new(|_cc| Ok(Box::new(GlucoseTrackerApp::new(db)))),
    )
}

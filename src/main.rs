// Release builds use the GUI subsystem, so no console window opens next to the calculator.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod converter;
mod dates;
mod engine;
mod graphing;
mod icon;
mod locale;
#[cfg(test)]
mod monkey;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CalcCraft")
            .with_inner_size([320.0, 500.0])
            .with_min_inner_size(app::MIN_SIZE)
            .with_icon(app::window_icon()),
        // Settings are saved, but the window always opens at the default size.
        persist_window: false,
        ..Default::default()
    };
    // The app name also names the settings folder: %APPDATA%\CalcCraft.
    eframe::run_native(
        "CalcCraft",
        options,
        Box::new(|cc| Ok(Box::new(app::CalculatorApp::new(cc)))),
    )
}

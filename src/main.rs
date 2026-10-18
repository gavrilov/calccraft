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

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui;

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

/// The web version (`index.html`, built with Trunk): the app draws on the page's canvas,
/// and the settings go to the browser's local storage.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    // Panics and warnings go to the browser's console.
    eframe::WebLogger::init(log::LevelFilter::Warn).ok();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("a page");
        let canvas = document
            .get_element_by_id("calccraft")
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("index.html has the canvas");
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(app::CalculatorApp::new(cc)))),
            )
            .await;
        // The loading message goes once the app runs, or tells why it couldn't start.
        if let Some(loading) = document.get_element_by_id("loading") {
            match started {
                Ok(()) => loading.remove(),
                Err(error) => {
                    loading.set_text_content(Some(&format!("CalcCraft couldn't start: {error:?}")))
                }
            }
        }
    });
}

// Release builds use the GUI subsystem, so no console window opens next to the calculator.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    calculator::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    calculator::start_web();
}

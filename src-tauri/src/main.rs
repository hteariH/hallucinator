#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ask;
mod download;
mod engine;
mod models;
mod settings;
mod vram;

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running LLM Calculator");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ask;
mod commands;
mod download;
mod engine;
mod models;
mod settings;
mod vram;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(commands::AppState::new(app.handle())?);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move { commands::boot(&handle).await });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::download_model,
            commands::delete_model,
            commands::select_model,
            commands::save_sampling,
            commands::save_personality,
            commands::restart_engine,
            commands::ask,
            commands::cancel_ask,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Hallucinator")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<commands::AppState>().kill_engine_now();
            }
        });
}

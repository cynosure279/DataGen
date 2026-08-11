// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            native::commands::detect_compilers,
            native::commands::generate_data,
            native::commands::compile_code,
            native::commands::run_binary,
            native::commands::generate_and_run,
            native::commands::save_files,
        ])
        .run(tauri::generate_context!())
        .unwrap();
}
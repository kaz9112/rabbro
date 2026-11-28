pub mod commands;
pub mod domain;
pub mod export;
pub mod engine;
pub mod models;
pub mod storage;

use models::catalog::StaticDatabase;
use models::state::ProjectState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use storage::autosave::load_autosave;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

pub struct AppState {
    pub db: StaticDatabase,
    pub project: Mutex<ProjectState>,
    pub dirty: AtomicBool,
    pub last_file_time: Mutex<Option<String>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = StaticDatabase::load_embedded().expect("Failed to load embedded AHS database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState {
            db,
            project: Mutex::new(ProjectState::default()),
            dirty: AtomicBool::new(false),
            last_file_time: Mutex::new(None),
        })
        .setup(|app| {
            println!("🦀 [BACKEND] Tauri setup started...");
            let handle = app.handle();
            let state = handle.state::<AppState>();
            
            println!("🦀 [BACKEND] Loading autosave...");
            let loaded = load_autosave(handle, &state.db);
            println!("🦀 [BACKEND] Autosave loaded successfully! Lines: {}", loaded.lines.len());
            
            if let Ok(mut proj) = state.project.lock() {
                *proj = loaded;
            }
            println!("🦀 [BACKEND] Setup complete!");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                let is_dirty = state.dirty.load(Ordering::Relaxed);
                let has_lines = state
                    .project
                    .lock()
                    .map(|p| !p.lines.is_empty())
                    .unwrap_or(false);

                if is_dirty && has_lines {
                    api.prevent_close();
                    let win = window.clone();
                    window
                        .dialog()
                        .message("Perubahan proyek belum disimpan ke file (.json). Tetap tutup aplikasi?")
                        .title("Konfirmasi Keluar")
                        .kind(MessageDialogKind::Warning)
                        .buttons(MessageDialogButtons::YesNo)
                        .show(move |confirmed| {
                            if confirmed {
                                let _ = win.destroy();
                            }
                        });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::boq_cmd::get_snapshot,
            commands::boq_cmd::update_meta,
            commands::boq_cmd::add_boq_line,
            commands::boq_cmd::duplicate_boq_line,
            commands::boq_cmd::update_boq_line,
            commands::boq_cmd::delete_boq_line,
            commands::boq_cmd::clear_boq,
            commands::hd_cmd::get_hd_view,
            commands::hd_cmd::set_hd_override,
            commands::hd_cmd::reset_all_hd,
            commands::hd_cmd::add_user_hd,
            commands::hd_cmd::update_user_hd,
            commands::hd_cmd::delete_user_hd,
            commands::ahs_cmd::get_ahs_editor_view,
            commands::ahs_cmd::commit_ahs_cell,
            commands::ahs_cmd::reset_ahs_overrides,
            commands::ahs_cmd::relink_hd_in_ahs,
            commands::ahs_cmd::get_user_ahs_list,
            commands::ahs_cmd::save_user_ahs,
            commands::ahs_cmd::delete_user_ahs,
            commands::file_cmd::save_project_dialog,
            commands::file_cmd::open_project_dialog,
            commands::file_cmd::export_xlsx_dialog,
            commands::file_cmd::export_csv_dialog,
            commands::file_cmd::get_boq_tsv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
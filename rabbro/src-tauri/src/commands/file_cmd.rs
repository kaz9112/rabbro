use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use super::common::{build_snapshot, AppSnapshot};
use crate::domain::ahs_builder::build_all_ahs_items;
use crate::domain::boq::{build_all_catalog, build_boq_summary, effective_divs};
use crate::export::csv::{build_boq_tsv, write_boq_csv};
use crate::export::xlsx::export_xlsx;
use crate::storage::autosave::save_autosave;
use crate::storage::project_io::{default_export_basename, open_project_json, save_project_json};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct OpenProjectResult {
    pub snapshot: AppSnapshot,
    pub filename: String,
    pub skipped: usize,
}

#[tauri::command]
pub async fn save_project_dialog(
    app: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<Option<AppSnapshot>, String> {
    let (default_name, proj_clone) = {
        let proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        (format!("{}.json", default_export_basename(&proj)), proj.clone())
    };

    let picked = app
        .dialog()
        .file()
        .add_filter("Proyek RAB", &["json"])
        .set_file_name(&default_name)
        .blocking_save_file();

    let Some(fp) = picked else {
        return Ok(None);
    };
    let path = fp.into_path().map_err(|e| e.to_string())?;
    save_project_json(&path, &proj_clone)?;

    app_state.dirty.store(false, Ordering::Relaxed);
    if let Ok(mut lt) = app_state.last_file_time.lock() {
        *lt = Some(path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string());
    }

    Ok(Some(build_snapshot(&app_state)?))
}

#[tauri::command]
pub async fn open_project_dialog(
    app: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<Option<OpenProjectResult>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("Proyek RAB", &["json"])
        .blocking_pick_file();

    let Some(fp) = picked else {
        return Ok(None);
    };
    let path = fp.into_path().map_err(|e| e.to_string())?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("proyek.json")
        .to_string();

    let (clean, skipped) = open_project_json(&path, &app_state.db)?;
    save_autosave(&app, &clean)?;

    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        *proj = clean;
    }
    app_state.dirty.store(false, Ordering::Relaxed);
    if let Ok(mut lt) = app_state.last_file_time.lock() {
        *lt = Some(filename.clone());
    }

    Ok(Some(OpenProjectResult {
        snapshot: build_snapshot(&app_state)?,
        filename,
        skipped,
    }))
}

#[tauri::command]
pub async fn export_xlsx_dialog(
    app: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let (default_name, proj_clone, ahs, summary) = {
        let proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        if proj.lines.is_empty() {
            return Err("BoQ masih kosong".into());
        }
        let ahs = build_all_ahs_items(&app_state.db, &proj)?;
        let catalog = build_all_catalog(&app_state.db, &ahs);
        let divs = effective_divs(&app_state.db, &proj);
        let summary = build_boq_summary(&proj, &ahs, &catalog, &divs);
        (
            format!("{}.xlsx", default_export_basename(&proj)),
            proj.clone(),
            ahs,
            summary,
        )
    };

    let picked = app
        .dialog()
        .file()
        .add_filter("Excel Workbook", &["xlsx"])
        .set_file_name(&default_name)
        .blocking_save_file();

    let Some(fp) = picked else {
        return Ok(false);
    };
    let path = fp.into_path().map_err(|e| e.to_string())?;
    export_xlsx(&path, &app_state.db, &proj_clone, &ahs, &summary)?;
    Ok(true)
}

#[tauri::command]
pub async fn export_csv_dialog(
    app: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    let (default_name, proj_clone, summary) = {
        let proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let ahs = build_all_ahs_items(&app_state.db, &proj)?;
        let catalog = build_all_catalog(&app_state.db, &ahs);
        let divs = effective_divs(&app_state.db, &proj);
        let summary = build_boq_summary(&proj, &ahs, &catalog, &divs);
        (
            format!("{}.csv", default_export_basename(&proj)),
            proj.clone(),
            summary,
        )
    };

    let picked = app
        .dialog()
        .file()
        .add_filter("CSV (Comma Separated)", &["csv"])
        .set_file_name(&default_name)
        .blocking_save_file();

    let Some(fp) = picked else {
        return Ok(false);
    };
    let path = fp.into_path().map_err(|e| e.to_string())?;
    write_boq_csv(&path, &proj_clone, &summary)?;
    Ok(true)
}

#[tauri::command]
pub fn get_boq_tsv(app_state: State<AppState>) -> Result<String, String> {
    let proj = app_state
        .project
        .lock()
        .map_err(|_| "Gagal mengunci state proyek".to_string())?;
    let ahs = build_all_ahs_items(&app_state.db, &proj)?;
    let catalog = build_all_catalog(&app_state.db, &ahs);
    let divs = effective_divs(&app_state.db, &proj);
    let summary = build_boq_summary(&proj, &ahs, &catalog, &divs);
    Ok(build_boq_tsv(&summary))
}
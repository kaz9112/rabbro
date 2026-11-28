use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tauri::AppHandle;

use crate::domain::ahs_builder::build_all_ahs_items;
use crate::domain::boq::{build_all_catalog, build_boq_summary, effective_divs, BoqSummary, CatalogEntry};
use crate::models::state::ProjectState;
use crate::storage::autosave::save_autosave;
use crate::storage::project_io::default_export_basename;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub state: ProjectState,
    pub divs: HashMap<String, String>,
    pub catalog: Vec<CatalogEntry>,
    pub boq: BoqSummary,
    pub dirty: bool,
    pub last_file_time: Option<String>,
    pub default_filename: String,
    pub hd_count: usize,
    pub ov_count: usize,
    pub ua_count: usize,
}

pub fn build_snapshot(app_state: &AppState) -> Result<AppSnapshot, String> {
    let proj = app_state
        .project
        .lock()
        .map_err(|_| "Gagal mengunci state proyek".to_string())?;
    let ahs = build_all_ahs_items(&app_state.db, &proj)?;
    let catalog = build_all_catalog(&app_state.db, &ahs);
    let divs = effective_divs(&app_state.db, &proj);
    let boq = build_boq_summary(&proj, &ahs, &catalog, &divs);
    let dirty = app_state.dirty.load(Ordering::Relaxed);
    let last_file_time = app_state
        .last_file_time
        .lock()
        .ok()
        .and_then(|g| g.clone());
    let default_filename = default_export_basename(&proj);

    Ok(AppSnapshot {
        hd_count: proj.hd.len(),
        ov_count: proj.ov.len(),
        ua_count: proj.uahs.len(),
        state: proj.clone(),
        divs,
        catalog,
        boq,
        dirty,
        last_file_time,
        default_filename,
    })
}

pub fn commit_and_snapshot(app: &AppHandle, app_state: &AppState) -> Result<AppSnapshot, String> {
    app_state.dirty.store(true, Ordering::Relaxed);
    {
        let proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        save_autosave(app, &proj)?;
    }
    build_snapshot(app_state)
}
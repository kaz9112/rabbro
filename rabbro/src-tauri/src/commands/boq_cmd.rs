use serde::Deserialize;
use tauri::{AppHandle, State};

use super::common::{build_snapshot, commit_and_snapshot, AppSnapshot};
use crate::domain::ahs_builder::build_all_ahs_items;
use crate::domain::boq::{build_all_catalog, effective_divs, norm_g};
use crate::models::state::BoqLine;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct UpdateMetaPayload {
    pub title: Option<String>,
    pub sub: Option<String>,
    pub proj: Option<String>,
    pub company: Option<String>,
    pub by: Option<String>,
    pub date: Option<String>,
    pub rev: Option<String>,
    pub fname: Option<String>,
    pub basis: Option<String>,
    pub ppn: Option<f64>,
    pub theme: Option<String>,
    pub font: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBoqLinePayload {
    pub id: u32,
    pub u: Option<String>,
    pub s: Option<String>,
    pub qty: Option<f64>,
    pub price: Option<f64>,
    pub note: Option<String>,
    pub dv: Option<String>,
    pub g: Option<String>,
}

#[tauri::command]
pub fn get_snapshot(app_state: State<AppState>) -> Result<AppSnapshot, String> {
    println!("🦀 [BACKEND] >> get_snapshot command received from frontend");
    match build_snapshot(&app_state) {
        Ok(snap) => {
            println!("🦀 [BACKEND] << get_snapshot finished successfully! (BoQ rows: {})", snap.boq.rows.len());
            Ok(snap)
        }
        Err(e) => {
            eprintln!("🦀 [BACKEND] !! get_snapshot ERROR: {}", e);
            Err(e)
        }
    }
}


#[tauri::command]
pub fn update_meta(
    app: AppHandle,
    app_state: State<AppState>,
    payload: UpdateMetaPayload,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        if let Some(v) = payload.title { proj.title = v; }
        if let Some(v) = payload.sub { proj.sub = v; }
        if let Some(v) = payload.proj { proj.proj = v; }
        if let Some(v) = payload.company { proj.company = v; }
        if let Some(v) = payload.by { proj.by = v; }
        if let Some(v) = payload.date { proj.date = v; }
        if let Some(v) = payload.rev { proj.rev = v; }
        if let Some(v) = payload.fname { proj.fname = v; }
        if let Some(v) = payload.basis {
            proj.basis = if v == "b" { "b".into() } else { "h".into() };
        }
        if let Some(v) = payload.ppn {
            if v.is_finite() { proj.ppn = v; }
        }
        if let Some(v) = payload.theme { proj.theme = v; }
        if let Some(v) = payload.font { proj.font = v; }
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn add_boq_line(
    app: AppHandle,
    app_state: State<AppState>,
    code: Option<String>,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let next_id = proj.lines.iter().map(|l| l.id).max().unwrap_or(0) + 1;

        if let Some(k) = code {
            let ahs = build_all_ahs_items(&app_state.db, &proj)?;
            let catalog = build_all_catalog(&app_state.db, &ahs);
            let item = catalog
                .into_iter()
                .find(|c| c.k == k)
                .ok_or_else(|| format!("Item '{}' tidak ditemukan", k))?;
            proj.lines.push(BoqLine {
                id: next_id,
                k: item.k,
                u: item.u,
                s: item.s,
                qty: if item.m { 1.0 } else { 0.0 },
                note: String::new(),
                custom: false,
                man: item.m,
                price: 0.0,
                dv: String::new(),
                g: String::new(),
            });
        } else {
            proj.lines.push(BoqLine {
                id: next_id,
                k: String::new(),
                u: String::new(),
                s: "ls".to_string(),
                qty: 1.0,
                note: String::new(),
                custom: true,
                man: false,
                price: 0.0,
                dv: String::new(),
                g: String::new(),
            });
        }
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn duplicate_boq_line(
    app: AppHandle,
    app_state: State<AppState>,
    id: u32,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let next_id = proj.lines.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        let src = proj
            .lines
            .iter()
            .find(|l| l.id == id)
            .cloned()
            .ok_or_else(|| "Baris tidak ditemukan".to_string())?;
        let mut copy = src;
        copy.id = next_id;
        copy.qty = 0.0;
        copy.note.clear();
        proj.lines.push(copy);
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn update_boq_line(
    app: AppHandle,
    app_state: State<AppState>,
    payload: UpdateBoqLinePayload,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let divs = effective_divs(&app_state.db, &proj);
        let line = proj
            .lines
            .iter_mut()
            .find(|l| l.id == payload.id)
            .ok_or_else(|| "Baris tidak ditemukan".to_string())?;

        if let Some(u) = payload.u { line.u = u; }
        if let Some(s) = payload.s { line.s = s; }
        if let Some(qty) = payload.qty {
            line.qty = if qty.is_finite() { qty } else { 0.0 };
        }
        if let Some(price) = payload.price {
            line.price = if price.is_finite() { price } else { 0.0 };
        }
        if let Some(note) = payload.note { line.note = note; }
        if let Some(dv) = payload.dv {
            line.dv = if divs.contains_key(&dv) { dv } else { String::new() };
            line.g.clear();
        }
        if let Some(g) = payload.g {
            line.g = if line.dv.is_empty() {
                String::new()
            } else {
                norm_g(g.trim())
            };
        }
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn delete_boq_line(
    app: AppHandle,
    app_state: State<AppState>,
    id: u32,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        proj.lines.retain(|l| l.id != id);
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn clear_boq(app: AppHandle, app_state: State<AppState>) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        proj.lines.clear();
    }
    commit_and_snapshot(&app, &app_state)
}
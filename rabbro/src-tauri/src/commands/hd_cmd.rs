use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::common::{commit_and_snapshot, AppSnapshot};
use crate::domain::ahs_builder::{build_all_ahs_items, effective_hd_rows, hd_is_used};
use crate::engine::evaluator::FormulaEngine;
use crate::engine::recalc::{build_effective_cells, effective_names};
use crate::models::state::{ProjectState, UserHdItem};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct HdRowView {
    pub r: u32,
    pub k: String,
    pub n: String,
    pub u: String,
    pub c: String,
    pub f: bool,
    pub usr: bool,
    pub orig: f64,
    pub cur: f64,
    pub is_chg: bool,
}

#[derive(Debug, Serialize)]
pub struct ImpactedAhsView {
    pub k: String,
    pub u: String,
    pub s: String,
    pub before: f64,
    pub after: f64,
}

#[derive(Debug, Serialize)]
pub struct HdView {
    pub rows: Vec<HdRowView>,
    pub impacted: Vec<ImpactedAhsView>,
    pub hd_count: usize,
    pub total_ahs: usize,
}

#[derive(Debug, Deserialize)]
pub struct AddUserHdPayload {
    pub cat: String,
    pub k: String,
    pub n: String,
    pub u: String,
    pub v: f64,
    pub c: String,
}

#[tauri::command]
pub fn get_hd_view(app_state: State<AppState>) -> Result<HdView, String> {
    let proj = app_state
        .project
        .lock()
        .map_err(|_| "Gagal mengunci state proyek".to_string())?;
    let hd_meta = effective_hd_rows(&app_state.db, &proj);
    let names = effective_names(&app_state.db);

    let mut base_state = ProjectState::default();
    base_state.uhd = proj.uhd.clone();
    let base_cells = build_effective_cells(&app_state.db, &base_state);
    let mut base_eng = FormulaEngine::new(&base_cells, &names);

    let eff_cells = build_effective_cells(&app_state.db, &proj);
    let mut cur_eng = FormulaEngine::new(&eff_cells, &names);

    let mut rows = Vec::with_capacity(hd_meta.len());
    for meta in hd_meta {
        let addr = format!("D{}", meta.r);
        let orig = base_eng.get_num("hd", &addr).unwrap_or(0.0);
        let cur = cur_eng.get_num("hd", &addr).unwrap_or(0.0);
        let is_chg = proj.hd.contains_key(&meta.r.to_string());
        rows.push(HdRowView {
            r: meta.r,
            k: meta.k,
            n: meta.n,
            u: meta.u,
            c: meta.c,
            f: meta.f,
            usr: meta.usr,
            orig,
            cur,
            is_chg,
        });
    }

    let ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
    let use_bare = proj.basis == "b";
    let mut impacted = Vec::new();
    for it in &ahs_items {
        let before = if use_bare { it.b0 } else { it.h0 };
        let after = if use_bare { it.b } else { it.h };
        if (before - after).abs() > 0.5 {
            impacted.push(ImpactedAhsView {
                k: it.k.clone(),
                u: it.u.clone(),
                s: it.s.clone(),
                before,
                after,
            });
        }
    }

    Ok(HdView {
        rows,
        impacted,
        hd_count: proj.hd.len(),
        total_ahs: ahs_items.len(),
    })
}

#[tauri::command]
pub fn set_hd_override(
    app: AppHandle,
    app_state: State<AppState>,
    r: u32,
    value: Option<f64>,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let key = r.to_string();
        let orig = app_state
            .db
            .payload
            .cells
            .get("hd")
            .and_then(|m| m.get(&format!("D{}", r)))
            .and_then(|v| v.as_f64())
            .or_else(|| proj.uhd.iter().find(|u| u.r == r).map(|u| u.v));

        match value {
            Some(v) if v.is_finite() && orig.map(|o| (o - v).abs() > 1e-12).unwrap_or(true) => {
                proj.hd.insert(key, v);
            }
            _ => {
                proj.hd.remove(&key);
            }
        }
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn reset_all_hd(app: AppHandle, app_state: State<AppState>) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        proj.hd.clear();
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn add_user_hd(
    app: AppHandle,
    app_state: State<AppState>,
    payload: AddUserHdPayload,
) -> Result<AppSnapshot, String> {
    let k = payload.k.trim().to_string();
    let n = payload.n.trim().to_string();
    let u = payload.u.trim().to_string();
    let cat = payload.cat.chars().next().unwrap_or('M');

    let valid_code = k.len() >= 3
        && k.len() <= 10
        && k.starts_with(cat)
        && k.as_bytes()[1] == b'.'
        && k[2..].chars().all(|c| c.is_ascii_alphanumeric());
    if !valid_code {
        return Err(format!(
            "Kode harus diawali {}. lalu 1 sampai 8 huruf atau angka",
            cat
        ));
    }
    if n.is_empty() {
        return Err("Uraian wajib diisi".into());
    }
    if u.is_empty() || u.chars().count() > 12 {
        return Err("Satuan wajib diisi (maks 12 karakter)".into());
    }
    if !payload.v.is_finite() || payload.v < 0.0 {
        return Err("Isi harga satuan (angka 0 atau lebih)".into());
    }

    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let existing = effective_hd_rows(&app_state.db, &proj);
        if existing
            .iter()
            .any(|x| x.k.eq_ignore_ascii_case(&k))
        {
            return Err(format!("Kode {} sudah dipakai", k));
        }

        let id = proj.uhnext;
        proj.uhnext += 1;
        proj.uhd.push(UserHdItem {
            id,
            r: 100000 + id,
            k,
            n: n.chars().take(160).collect(),
            u,
            c: payload.c.trim().chars().take(160).collect(),
            v: payload.v,
        });
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn update_user_hd(
    app: AppHandle,
    app_state: State<AppState>,
    r: u32,
    n: Option<String>,
    u: Option<String>,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        if let Some(item) = proj.uhd.iter_mut().find(|z| z.r == r) {
            if let Some(name) = n {
                let t = name.trim();
                if !t.is_empty() {
                    item.n = t.chars().take(160).collect();
                }
            }
            if let Some(unit) = u {
                let t: String = unit.trim().chars().take(12).collect();
                if !t.is_empty() {
                    item.u = t;
                }
            }
        }
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn delete_user_hd(
    app: AppHandle,
    app_state: State<AppState>,
    r: u32,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        if hd_is_used(r, &proj) {
            return Err("Item ini dipakai di AHS buatan sendiri. Ganti dulu tautannya.".into());
        }
        proj.uhd.retain(|z| z.r != r);
        proj.hd.remove(&r.to_string());
    }
    commit_and_snapshot(&app, &app_state)
}
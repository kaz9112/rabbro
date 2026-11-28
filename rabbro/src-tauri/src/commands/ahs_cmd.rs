use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::{AppHandle, State};

use super::common::{commit_and_snapshot, AppSnapshot};
use crate::domain::ahs_builder::{
    build_all_ahs_items, derive_user_ahs, effective_hd_rows, gen_spec_user_ahs, hdr_line,
    next_derived_code, resolve_hd, STRIDE,
};
use crate::domain::boq::{build_all_catalog, effective_divs, norm_g};
use crate::engine::evaluator::{EvalValue, FormulaEngine};
use crate::engine::parser::{parse_user_input, ParsedInput};
use crate::engine::recalc::{build_effective_cells, effective_names, recalc_items};
use crate::models::catalog::CellValue;
use crate::models::state::UserAhsSpec;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct AhsSelectorItem {
    pub idx: usize,
    pub k: String,
    pub u: String,
    pub dv: String,
    pub dv_name: String,
}

#[derive(Debug, Serialize)]
pub struct AhsGridCell {
    pub addr: String,
    pub is_str: bool,
    pub is_formula: bool,
    pub is_modified: bool,
    pub display: String,
    pub num_val: Option<f64>,
    pub raw_input: String,
    pub orig_raw: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum AhsGridRow {
    #[serde(rename = "banner")]
    Banner { text: String, is_section: bool },
    #[serde(rename = "header")]
    Header { cols: Vec<String> },
    #[serde(rename = "data")]
    Data { is_total: bool, cells: Vec<Option<AhsGridCell>> },
}

#[derive(Debug, Serialize)]
pub struct RelinkRowView {
    pub addr: String,
    pub label: String,
    pub current_hd_label: String,
    pub is_pure_ref: bool,
    pub raw_formula: String,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct AhsEditorView {
    pub selector: Vec<AhsSelectorItem>,
    pub selected_idx: usize,
    pub k: String,
    pub unit: String,
    pub bare: f64,
    pub bare0: f64,
    pub total: f64,
    pub total0: f64,
    pub hours: f64,
    pub hours0: f64,
    pub rows: Vec<AhsGridRow>,
    pub relink_rows: Vec<RelinkRowView>,
    pub is_user: bool,
    pub is_spec: bool,
    pub ov_count: usize,
}

#[derive(Debug, Serialize)]
pub struct UserAhsListItem {
    pub id: u32,
    pub k: String,
    pub u: String,
    pub s: String,
    pub dv: String,
    pub g: String,
    pub from: String,
    pub is_spec: bool,
    pub spec: Option<UserAhsSpec>,
    pub price: f64,
    pub used_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct SaveUserAhsPayload {
    pub mode: String, // "new" | "der" | "edit" | "info"
    pub id: Option<u32>,
    pub src_k: Option<String>,
    pub k: String,
    pub u: String,
    pub s: String,
    pub dv: String,
    pub new_dv_code: Option<String>,
    pub new_dv_name: Option<String>,
    pub g: String,
    pub note: Option<String>,
    pub spec: Option<UserAhsSpec>,
}

fn to_hd_short(s: &str) -> String {
    s.replace("'Harga Dasar'!", "HD!")
}

fn is_pure_hd_ref(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix("='Harga Dasar'!D") {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

#[tauri::command]
pub fn get_ahs_editor_view(
    app_state: State<AppState>,
    idx: usize,
) -> Result<AhsEditorView, String> {
    let proj = app_state
        .project
        .lock()
        .map_err(|_| "Gagal mengunci state proyek".to_string())?;
    let ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
    let divs = effective_divs(&app_state.db, &proj);
    let sel_idx = if idx < ahs_items.len() { idx } else { 0 };
    let b = &ahs_items[sel_idx];

    let selector: Vec<AhsSelectorItem> = ahs_items
        .iter()
        .enumerate()
        .map(|(i, it)| AhsSelectorItem {
            idx: i,
            k: it.k.clone(),
            u: it.u.clone(),
            dv: it.dv.clone(),
            dv_name: divs.get(&it.dv).cloned().unwrap_or_default(),
        })
        .collect();

    let names = effective_names(&app_state.db);
    let mut base_state = crate::models::state::ProjectState::default();
    base_state.uahs = proj.uahs.clone();
    base_state.uhd = proj.uhd.clone();
    let orig_cells = build_effective_cells(&app_state.db, &base_state);
    let mut orig_eng = FormulaEngine::new(&orig_cells, &names);

    let eff_cells = build_effective_cells(&app_state.db, &proj);
    let mut cur_eng = FormulaEngine::new(&eff_cells, &names);

    let bare = cur_eng.get_num(&b.d, &b.cell_ref.b).unwrap_or(0.0);
    let bare0 = orig_eng.get_num(&b.d, &b.cell_ref.b).unwrap_or(0.0);
    let total = cur_eng.get_num(&b.d, &b.cell_ref.h).unwrap_or(0.0);
    let total0 = orig_eng.get_num(&b.d, &b.cell_ref.h).unwrap_or(0.0);
    let hours = cur_eng.get_num(&b.d, &b.cell_ref.j).unwrap_or(0.0);
    let hours0 = orig_eng.get_num(&b.d, &b.cell_ref.j).unwrap_or(0.0);

    let orig_sheet = orig_cells.get(&b.d);
    let cols = ["A", "B", "C", "D", "E", "F", "G"];
    let mut rows = Vec::new();

    for r in b.start..=b.end {
        let mut present = Vec::new();
        for c in &cols {
            let addr = format!("{}{}", c, r);
            if let Some(orig_map) = orig_sheet {
                if orig_map.contains_key(&addr) {
                    present.push((c, addr));
                }
            }
        }
        if present.is_empty() {
            continue;
        }
        let is_plain_str = |addr: &str| -> bool {
            matches!(
                orig_sheet.and_then(|m| m.get(addr)),
                Some(CellValue::Text(s)) if !s.starts_with('=')
            )
        };
        let all_str = present.iter().all(|(_, a)| is_plain_str(a));
        if all_str && present.len() == 1 && present[0].1 == format!("A{}", r) {
            if let Some(CellValue::Text(t)) = orig_sheet.and_then(|m| m.get(&present[0].1)) {
                let is_sec = t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                    && t.contains('.');
                rows.push(AhsGridRow::Banner {
                    text: t.clone(),
                    is_section: is_sec,
                });
            }
            continue;
        }
        if all_str {
            let hdr_cols = (0..6)
                .map(|i| {
                    let addr = format!("{}{}", cols[i], r);
                    match orig_sheet.and_then(|m| m.get(&addr)) {
                        Some(CellValue::Text(s)) => s.clone(),
                        _ => String::new(),
                    }
                })
                .collect();
            rows.push(AhsGridRow::Header { cols: hdr_cols });
            continue;
        }

        let b_lbl = match orig_sheet.and_then(|m| m.get(&format!("B{}", r))) {
            Some(CellValue::Text(s)) => s.as_str(),
            _ => "",
        };
        let is_total = b_lbl.starts_with("Jumlah")
            || b_lbl.starts_with("Total")
            || b_lbl.starts_with("TOTAL")
            || b_lbl.starts_with("Dibulatkan");

        let mut row_cells = Vec::with_capacity(6);
        for i in 0..6 {
            let addr = format!("{}{}", cols[i], r);
            let Some(orig_val) = orig_sheet.and_then(|m| m.get(&addr)) else {
                row_cells.push(None);
                continue;
            };
            let orig_raw_str = match orig_val {
                CellValue::Number(n) => format!("{}", n),
                CellValue::Text(s) => to_hd_short(s),
            };
            if let CellValue::Text(s) = orig_val {
                if !s.starts_with('=') {
                    row_cells.push(Some(AhsGridCell {
                        addr,
                        is_str: true,
                        is_formula: false,
                        is_modified: false,
                        display: s.clone(),
                        num_val: None,
                        raw_input: s.clone(),
                        orig_raw: orig_raw_str,
                    }));
                    continue;
                }
            }

            let ov_key = format!("{}:{}", b.d, addr);
            let ov_val = proj.ov.get(&ov_key);
            let is_modified = ov_val.is_some();
            let raw_input = match ov_val {
                Some(CellValue::Number(n)) => format!("{}", n),
                Some(CellValue::Text(s)) => to_hd_short(s),
                None => orig_raw_str.clone(),
            };
            let is_formula = orig_val.is_formula();
            let (display, num_val) = match cur_eng.get(&b.d, &addr) {
                Ok(EvalValue::Number(n)) => (format!("{}", n), Some(n)),
                Ok(EvalValue::Text(s)) => (s, None),
                Err(_) => ("#ERR".to_string(), None),
            };

            row_cells.push(Some(AhsGridCell {
                addr,
                is_str: false,
                is_formula,
                is_modified,
                display,
                num_val,
                raw_input,
                orig_raw: orig_raw_str,
            }));
        }
        rows.push(AhsGridRow::Data {
            is_total,
            cells: row_cells,
        });
    }

    // Relink table for derived user AHS
    let mut relink_rows = Vec::new();
    if b.usr.is_some() && !b.spec {
        let hd_meta = effective_hd_rows(&app_state.db, &proj);
        for r in b.start..=b.end {
            for col in ["D", "E"] {
                let addr = format!("{}{}", col, r);
                let ov_key = format!("usr:{}", addr);
                let val = proj
                    .ov
                    .get(&ov_key)
                    .or_else(|| orig_sheet.and_then(|m| m.get(&addr)));
                let Some(CellValue::Text(f)) = val else { continue };
                if !f.starts_with('=') || !f.contains("'Harga Dasar'!") {
                    continue;
                }
                // Find first L/M/E HD row referenced in formula
                let matched_hd = hd_meta.iter().find(|h| {
                    matches!(h.k.chars().next(), Some('L' | 'M' | 'E'))
                        && (f.contains(&format!("'Harga Dasar'!D{}", h.r))
                            || f.contains(&format!("'Harga Dasar'!$D${}", h.r)))
                });
                if let Some(hd) = matched_hd {
                    let row_lbl = match orig_sheet.and_then(|m| m.get(&format!("B{}", r))) {
                        Some(CellValue::Text(s)) => s.clone(),
                        _ => String::new(),
                    };
                    let price = cur_eng.get_num("usr", &addr).unwrap_or(0.0);
                    relink_rows.push(RelinkRowView {
                        addr,
                        label: row_lbl,
                        current_hd_label: format!("{} | {}", hd.k, hd.n),
                        is_pure_ref: is_pure_hd_ref(f),
                        raw_formula: to_hd_short(f),
                        price,
                    });
                }
            }
        }
    }

    Ok(AhsEditorView {
        selector,
        selected_idx: sel_idx,
        k: b.k.clone(),
        unit: b.s.clone(),
        bare,
        bare0,
        total,
        total0,
        hours,
        hours0,
        rows,
        relink_rows,
        is_user: b.usr.is_some(),
        is_spec: b.spec,
        ov_count: proj.ov.len(),
    })
}

#[tauri::command]
pub fn commit_ahs_cell(
    app: AppHandle,
    app_state: State<AppState>,
    idx: usize,
    addr: String,
    text: String,
) -> Result<AppSnapshot, String> {
    let names = effective_names(&app_state.db);
    let parsed = parse_user_input(&text, &names)?;

    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let mut ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
        let b = ahs_items
            .get(idx)
            .ok_or_else(|| "AHS tidak ditemukan".to_string())?
            .clone();

        let ov_key = format!("{}:{}", b.d, addr);
        let prev = proj.ov.get(&ov_key).cloned();

        let mut base_state = crate::models::state::ProjectState::default();
        base_state.uahs = proj.uahs.clone();
        let base_cells = build_effective_cells(&app_state.db, &base_state);
        let orig_val = base_cells.get(&b.d).and_then(|m| m.get(&addr)).cloned();

        match parsed {
            ParsedInput::Delete => {
                proj.ov.remove(&ov_key);
            }
            ParsedInput::Number(n) => {
                if orig_val.as_ref() == Some(&CellValue::Number(n)) {
                    proj.ov.remove(&ov_key);
                } else {
                    proj.ov.insert(ov_key.clone(), CellValue::Number(n));
                }
            }
            ParsedInput::Formula(f) => {
                if orig_val.as_ref() == Some(&CellValue::Text(f.clone())) {
                    proj.ov.remove(&ov_key);
                } else {
                    proj.ov.insert(ov_key.clone(), CellValue::Text(f));
                }
            }
        }

        let eff_cells = build_effective_cells(&app_state.db, &proj);
        if let Err(err) = recalc_items(
            &mut ahs_items,
            &eff_cells,
            &names,
            Some((&b.d, b.start, b.end)),
        ) {
            if let Some(p) = prev {
                proj.ov.insert(ov_key, p);
            } else {
                proj.ov.remove(&ov_key);
            }
            return Err(format!("Perubahan ditolak: {}", err));
        }
    }

    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn reset_ahs_overrides(
    app: AppHandle,
    app_state: State<AppState>,
    idx: Option<usize>,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        if let Some(i) = idx {
            let ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
            if let Some(b) = ahs_items.get(i) {
                let prefix = format!("{}:", b.d);
                proj.ov.retain(|k, _| {
                    if let Some(addr) = k.strip_prefix(&prefix) {
                        let r: u32 = addr
                            .chars()
                            .skip_while(|c| !c.is_ascii_digit())
                            .collect::<String>()
                            .parse()
                            .unwrap_or(0);
                        r < b.start || r > b.end
                    } else {
                        true
                    }
                });
            }
        } else {
            proj.ov.clear();
        }
    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn relink_hd_in_ahs(
    app: AppHandle,
    app_state: State<AppState>,
    idx: usize,
    addr: String,
    new_hd_text: String,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let hd_meta = effective_hd_rows(&app_state.db, &proj);
        let hd = resolve_hd(&new_hd_text, None, &hd_meta).ok_or_else(|| {
            "Pilih item dari daftar Harga Dasar (upah, bahan, atau alat)".to_string()
        })?;

        let ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
        let item = ahs_items
            .get(idx)
            .ok_or_else(|| "AHS tidak ditemukan".to_string())?;
        let ua_id = item.usr.ok_or_else(|| "Bukan AHS buatan sendiri".to_string())?;

        let ua = proj
            .uahs
            .iter_mut()
            .find(|z| z.id == ua_id)
            .ok_or_else(|| "Data AHS pengguna tidak ditemukan".to_string())?;

        let r: u32 = addr
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .map_err(|_| "Alamat sel tidak valid".to_string())?;

        ua.cells.insert(
            addr.clone(),
            CellValue::Text(format!("='Harga Dasar'!D{}", hd.r)),
        );

        let old_b = match ua.cells.get(&format!("B{}", r)) {
            Some(CellValue::Text(s)) => s.clone(),
            _ => String::new(),
        };
        let prefix = if old_b.starts_with("Material: ") {
            "Material: "
        } else if old_b.starts_with("Equipment: ") {
            "Equipment: "
        } else {
            ""
        };
        ua.cells
            .insert(format!("A{}", r), CellValue::Text(hd.k.clone()));
        ua.cells
            .insert(format!("B{}", r), CellValue::Text(format!("{}{}", prefix, hd.n)));
        if matches!(ua.cells.get(&format!("C{}", r)), Some(CellValue::Text(_))) {
            ua.cells
                .insert(format!("C{}", r), CellValue::Text(hd.u.clone()));
        }
        proj.ov.remove(&format!("usr:{}", addr));

    }
    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn get_user_ahs_list(app_state: State<AppState>) -> Result<Vec<UserAhsListItem>, String> {
    let proj = app_state
        .project
        .lock()
        .map_err(|_| "Gagal mengunci state proyek".to_string())?;
    let ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
    let use_bare = proj.basis == "b";

    let out = ahs_items
        .into_iter()
        .filter_map(|it| {
            let id = it.usr?;
            let ua = proj.uahs.iter().find(|z| z.id == id);
            let used_count = proj.lines.iter().filter(|l| l.k == it.k).count();
            Some(UserAhsListItem {
                id,
                k: it.k,
                u: it.u,
                s: it.s,
                dv: it.dv,
                g: it.g,
                from: it.from,
                is_spec: it.spec,
                spec: ua.and_then(|z| z.spec.clone()),
                price: if use_bare { it.b } else { it.h },
                used_count,
            })
        })
        .collect();
    Ok(out)
}

#[tauri::command]
pub fn save_user_ahs(
    app: AppHandle,
    app_state: State<AppState>,
    payload: SaveUserAhsPayload,
) -> Result<AppSnapshot, String> {
    let k = payload.k.trim().to_string();
    let u = payload.u.trim().to_string();
    let s = payload.s.trim().to_string();
    let g = norm_g(payload.g.trim());

    if k.is_empty() || k.chars().count() > 40 {
        return Err("Kode AHS wajib diisi (maks 40 karakter)".into());
    }
    if u.is_empty() {
        return Err("Uraian pekerjaan wajib diisi".into());
    }
    if s.is_empty() || s.chars().count() > 12 {
        return Err("Satuan wajib diisi (maks 12 karakter)".into());
    }
    if g.is_empty() {
        return Err("Subdivisi wajib diisi".into());
    }

    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let mut divs = effective_divs(&app_state.db, &proj);
        let ahs_items = build_all_ahs_items(&app_state.db, &proj)?;
        let catalog = build_all_catalog(&app_state.db, &ahs_items);
        let existing_codes: HashSet<String> = catalog.iter().map(|c| c.k.clone()).collect();

        if (payload.mode == "new" || payload.mode == "der") && existing_codes.contains(&k) {
            return Err(format!("Kode AHS '{}' sudah dipakai", k));
        }

        let dv = if payload.dv == "__new" {
            let nc = payload.new_dv_code.as_deref().unwrap_or("").trim().to_string();
            let nn = payload.new_dv_name.as_deref().unwrap_or("").trim().to_string();
            if nc.is_empty() || nc.len() > 4 || !nc.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err("Kode divisi baru: 1 sampai 4 huruf atau angka".into());
            }
            if divs.contains_key(&nc) {
                return Err(format!("Kode divisi '{}' sudah ada", nc));
            }
            if nn.is_empty() {
                return Err("Nama divisi baru wajib diisi".into());
            }
            let upper_name: String = nn.to_uppercase().chars().take(80).collect();
            proj.udivs.insert(nc.clone(), upper_name.clone());
            divs.insert(nc.clone(), upper_name);
            nc
        } else if divs.contains_key(&payload.dv) {
            payload.dv.clone()
        } else {
            return Err("Pilih divisi".into());
        };

        let hd_rows = effective_hd_rows(&app_state.db, &proj);

        match payload.mode.as_str() {
            "new" => {
                let spec = payload
                    .spec
                    .as_ref()
                    .ok_or_else(|| "Spesifikasi AHS kosong".to_string())?;
                if !(spec.out > 0.0) {
                    return Err("Daily output harus lebih dari 0".into());
                }
                if spec.crew.is_empty() && spec.mats.is_empty() && spec.eqs.is_empty() {
                    return Err("Tambahkan minimal satu komponen".into());
                }
                let id = proj.unext;
                let ua = gen_spec_user_ahs(id, k, u.clone(), s.clone(), dv, g, spec, &hd_rows, None)?;
                proj.unext += 1;
                proj.uahs.push(ua);
            }
            "der" => {
                let src_k = payload
                    .src_k
                    .as_deref()
                    .ok_or_else(|| "AHS asal tidak dipilih".to_string())?;
                let src = ahs_items
                    .iter()
                    .find(|x| x.k == src_k)
                    .ok_or_else(|| "AHS asal tidak ditemukan".to_string())?;
                let new_code = next_derived_code(src, &existing_codes);
                let eff_cells = build_effective_cells(&app_state.db, &proj);
                let names = effective_names(&app_state.db);
                let id = proj.unext;
                let ua = derive_user_ahs(
                    id,
                    src,
                    new_code,
                    u,
                    s,
                    dv,
                    g,
                    payload.note.as_deref().unwrap_or(""),
                    &eff_cells,
                    &names,
                )?;
                proj.unext += 1;
                proj.uahs.push(ua);
            }
            "edit" => {
                let id = payload.id.ok_or_else(|| "ID AHS tidak ditemukan".to_string())?;
                let spec = payload
                    .spec
                    .as_ref()
                    .ok_or_else(|| "Spesifikasi AHS kosong".to_string())?;
                if !(spec.out > 0.0) {
                    return Err("Daily output harus lebih dari 0".into());
                }
                let start = 5 + id * STRIDE;
                let updated = gen_spec_user_ahs(
                    id,
                    k.clone(),
                    u.clone(),
                    s.clone(),
                    dv,
                    g,
                    spec,
                    &hd_rows,
                    Some(start),
                )?;
                let end = updated.end;
                let ua = proj
                    .uahs
                    .iter_mut()
                    .find(|z| z.id == id)
                    .ok_or_else(|| "AHS tidak ditemukan".to_string())?;
                *ua = updated;
                proj.ov.retain(|key, _| {
                    if let Some(addr) = key.strip_prefix("usr:") {
                        let r: u32 = addr
                            .chars()
                            .skip_while(|c| !c.is_ascii_digit())
                            .collect::<String>()
                            .parse()
                            .unwrap_or(0);
                        r < start || r > end
                    } else {
                        true
                    }
                });
                for line in &mut proj.lines {
                    if !line.custom && !line.man && line.k == k {
                        line.u = u.clone();
                        line.s = s.clone();
                    }
                }
            }
            "info" => {
                let id = payload.id.ok_or_else(|| "ID AHS tidak ditemukan".to_string())?;
                let ua = proj
                    .uahs
                    .iter_mut()
                    .find(|z| z.id == id)
                    .ok_or_else(|| "AHS tidak ditemukan".to_string())?;
                ua.u = u.clone();
                ua.s = s.clone();
                ua.dv = dv.clone();
                ua.g = g.clone();
                ua.cells.insert(
                    format!("A{}", ua.start),
                    CellValue::Text(format!("{}  {}", ua.k, u)),
                );
                ua.cells.insert(
                    format!("A{}", ua.start + 2),
                    CellValue::Text(hdr_line(&dv, &g, &s)),
                );
                let target_k = ua.k.clone();
                for line in &mut proj.lines {
                    if !line.custom && !line.man && line.k == target_k {
                        line.u = u.clone();
                        line.s = s.clone();
                    }
                }
            }
            _ => return Err("Mode tidak dikenal".into()),
        }

        // Verify entire project still recalculates cleanly
        build_all_ahs_items(&app_state.db, &proj)?;
    }

    commit_and_snapshot(&app, &app_state)
}

#[tauri::command]
pub fn delete_user_ahs(
    app: AppHandle,
    app_state: State<AppState>,
    id: u32,
) -> Result<AppSnapshot, String> {
    {
        let mut proj = app_state
            .project
            .lock()
            .map_err(|_| "Gagal mengunci state proyek".to_string())?;
        let ua = proj
            .uahs
            .iter()
            .find(|z| z.id == id)
            .cloned()
            .ok_or_else(|| "AHS tidak ditemukan".to_string())?;
        let used = proj.lines.iter().filter(|l| l.k == ua.k).count();
        if used > 0 {
            return Err(format!(
                "AHS dipakai di {} baris BoQ. Hapus baris itu dulu.",
                used
            ));
        }
        proj.ov.retain(|key, _| {
            if let Some(addr) = key.strip_prefix("usr:") {
                let r: u32 = addr
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0);
                r < ua.start || r > ua.end
            } else {
                true
            }
        });
        proj.uahs.retain(|z| z.id != id);
    }
    commit_and_snapshot(&app, &app_state)
}
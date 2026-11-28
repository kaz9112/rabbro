use std::collections::HashMap;

use super::evaluator::{EvalValue, FormulaEngine};
use crate::models::catalog::{AhsItem, CellValue, StaticDatabase};
use crate::models::state::ProjectState;

pub fn build_effective_cells(
    db: &StaticDatabase,
    state: &ProjectState,
) -> HashMap<String, HashMap<String, CellValue>> {
    let mut cells = db.payload.cells.clone();

    // Ensure user AHS sheet exists
    let usr_sheet = cells.entry("usr".to_string()).or_default();
    for ua in &state.uahs {
        for (addr, val) in &ua.cells {
            usr_sheet.insert(addr.clone(), val.clone());
        }
    }

    // Apply user-added Harga Dasar base values
    if let Some(hd_sheet) = cells.get_mut("hd") {
        for uhd in &state.uhd {
            hd_sheet.insert(format!("D{}", uhd.r), CellValue::Number(uhd.v));
        }
        // Apply Harga Dasar price overrides
        for (row_str, val) in &state.hd {
            hd_sheet.insert(format!("D{}", row_str), CellValue::Number(*val));
        }
    }

    // Apply AHS cell overrides ("sheet:addr" -> CellValue)
    for (key, val) in &state.ov {
        if let Some((sh, addr)) = key.split_once(':') {
            if let Some(sheet_map) = cells.get_mut(sh) {
                sheet_map.insert(addr.to_string(), val.clone());
            }
        }
    }

    cells
}

pub fn effective_names(db: &StaticDatabase) -> HashMap<String, String> {
    let mut names = db.payload.names.clone();
    names
        .entry("AHS Buatan Pengguna".to_string())
        .or_insert_with(|| "usr".to_string());
    names
}

pub fn recalc_items(
    items: &mut [AhsItem],
    cells: &HashMap<String, HashMap<String, CellValue>>,
    names: &HashMap<String, String>,
    validate_block: Option<(&str, u32, u32)>,
) -> Result<(), String> {
    let mut eng = FormulaEngine::new(cells, names);

    for item in items.iter_mut() {
        let h = eng.get_num(&item.d, &item.cell_ref.h)?;
        let b = eng.get_num(&item.d, &item.cell_ref.b)?;
        if !h.is_finite() || !b.is_finite() {
            return Err("Hasil perhitungan AHS tidak valid".into());
        }
        item.h = h;
        item.b = b;
    }

    if let Some((sh, start, end)) = validate_block {
        let addrs: Vec<String> = cells
            .get(sh)
            .map(|m| {
                m.keys()
                    .filter(|a| {
                        a.get(1..)
                            .and_then(|r| r.parse::<u32>().ok())
                            .map(|r| r >= start && r <= end)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        for addr in addrs {
            if let EvalValue::Number(n) = eng.get(sh, &addr)? {
                if !n.is_finite() {
                    return Err("Hasil perhitungan sel tidak valid".into());
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_baseline_ahs_match_payload_h0_and_b0() {
        let db = StaticDatabase::load_embedded().unwrap();
        let state = ProjectState::default();
        let cells = build_effective_cells(&db, &state);
        let names = effective_names(&db);
        let mut items = db.payload.items.clone();

        recalc_items(&mut items, &cells, &names, None).expect("Recalc failed");

        for item in &items {
            assert!(
                (item.h - item.h0).abs() < 1e-4,
                "Mismatch h for {}: got {}, expected {}",
                item.k,
                item.h,
                item.h0
            );
            assert!(
                (item.b - item.b0).abs() < 1e-4,
                "Mismatch b for {}: got {}, expected {}",
                item.k,
                item.b,
                item.b0
            );
        }
    }
}
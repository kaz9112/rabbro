use rust_xlsxwriter::{
    Color, Format, FormatAlign, FormatBorder, Formula, Workbook,
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::csv::doc_rows;
use crate::domain::ahs_builder::effective_hd_rows;
use crate::domain::boq::{BoqRowView, BoqSummary};
use crate::engine::evaluator::{EvalValue, FormulaEngine};
use crate::engine::recalc::{build_effective_cells, effective_names};
use crate::models::catalog::{AhsItem, CellValue, StaticDatabase};
use crate::models::state::ProjectState;

struct ThemePalette {
    acc: u32,
    dv: u32,
    soft: u32,
}

fn theme_palette(theme: &str) -> ThemePalette {
    match theme {
        "light" => ThemePalette { acc: 0x6B7280, dv: 0x8A929D, soft: 0xEFF0F2 },
        "dark" => ThemePalette { acc: 0x2F343B, dv: 0x4B5563, soft: 0xD9DCE1 },
        "blue" => ThemePalette { acc: 0x1D4ED8, dv: 0x4272E6, soft: 0xE8EDFB },
        "red" => ThemePalette { acc: 0xB91C1C, dv: 0xD14B4B, soft: 0xF8E8E8 },
        _ => ThemePalette { acc: 0x1F6F5C, dv: 0x2E8B73, soft: 0xE3EFEA },
    }
}

fn excel_font(font: &str) -> &'static str {
    match font {
        "arial" => "Arial",
        "tahoma" => "Tahoma",
        "times" => "Times New Roman",
        _ => "Calibri",
    }
}

// fn col_to_idx(col: &str) -> u16 {
//     col.bytes().fold(0u16, |acc, b| acc * 26 + (b - b'A' + 1) as u16) - 1
// }

fn split_addr(addr: &str) -> (&str, u32) {
    let idx = addr.find(|c: char| c.is_ascii_digit()).unwrap_or(1);
    let (c, r) = addr.split_at(idx);
    (c, r.parse::<u32>().unwrap_or(1))
}

fn parse_cell_ref(chars: &[char], mut i: usize) -> Option<(String, String, String, u32, usize)> {
    let c_d = if i < chars.len() && chars[i] == '$' { i += 1; "$" } else { "" };
    let c_start = i;
    while i < chars.len() && chars[i].is_ascii_uppercase() && (i - c_start) < 2 {
        i += 1;
    }
    if i == c_start {
        return None;
    }
    let col: String = chars[c_start..i].iter().collect();

    let r_d = if i < chars.len() && chars[i] == '$' { i += 1; "$" } else { "" };
    let r_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == r_start {
        return None;
    }
    if i < chars.len() && (chars[i] == '(' || chars[i].is_ascii_alphabetic()) {
        return None;
    }
    let row: u32 = chars[r_start..i].iter().collect::<String>().parse().ok()?;
    Some((c_d.into(), col, r_d.into(), row, i))
}

fn extract_refs(formula: &str) -> Vec<(Option<String>, u32)> {
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;
    let mut refs = Vec::new();
    while i < chars.len() {
        let mut sh: Option<String> = None;
        let saved = i;
        if chars[i] == '\'' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '\'' {
                j += 1;
            }
            if j + 1 < chars.len() && chars[j + 1] == '!' {
                sh = Some(chars[i + 1..j].iter().collect());
                i = j + 2;
            } else {
                i += 1;
                continue;
            }
        }
        if let Some((_, _, _, row, next_i)) = parse_cell_ref(&chars, i) {
            refs.push((sh, row));
            i = next_i;
            continue;
        }
        if sh.is_some() {
            i = saved;
        }
        i += 1;
    }
    refs
}

fn remap_formula_ahs(
    formula: &str,
    cur_sheet: &str,
    names: &HashMap<String, String>,
    hd_map: &HashMap<u32, u32>,
    items: &[AhsItem],
    new_starts: &HashMap<String, u32>,
) -> String {
    let block_new_row = |sh: &str, r: u32| -> u32 {
        for it in items {
            if it.d == sh && r >= it.start && r <= it.end {
                if let Some(&ns) = new_starts.get(&it.k) {
                    return ns + (r - it.start);
                }
            }
        }
        r
    };

    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::with_capacity(formula.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        let mut sh_name: Option<String> = None;
        let saved = i;
        if chars[i] == '\'' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '\'' {
                j += 1;
            }
            if j + 1 < chars.len() && chars[j + 1] == '!' {
                sh_name = Some(chars[i + 1..j].iter().collect());
                i = j + 2;
            } else {
                out.push(chars[i]);
                i += 1;
                continue;
            }
        }
        if let Some((cd, col, rd, row, next_i)) = parse_cell_ref(&chars, i) {
            if sh_name.as_deref() == Some("Harga Dasar") {
                let nr = hd_map.get(&row).copied().unwrap_or(row);
                out.push_str(&format!("'Harga Dasar'!{}{}{}{}", cd, col, rd, nr));
            } else {
                let dd = sh_name
                    .as_deref()
                    .and_then(|n| names.get(n).map(String::as_str))
                    .unwrap_or(cur_sheet);
                let nr = block_new_row(dd, row);
                if let Some(sn) = &sh_name {
                    out.push_str(&format!("'{}'!{}{}{}{}", sn, cd, col, rd, nr));
                } else {
                    out.push_str(&format!("{}{}{}{}", cd, col, rd, nr));
                }
            }
            i = next_i;
            continue;
        }
        if sh_name.is_some() {
            i = saved;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn remap_formula_hd(formula: &str, hd_map: &HashMap<u32, u32>) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::with_capacity(formula.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some((cd, col, rd, row, next_i)) = parse_cell_ref(&chars, i) {
            let nr = hd_map.get(&row).copied().unwrap_or(row);
            out.push_str(&format!("{}{}{}{}", cd, col, rd, nr));
            i = next_i;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn sheet_name_trunc(name: &str) -> String {
    name.chars().take(31).collect()
}

pub fn export_xlsx(
    path: &Path,
    db: &StaticDatabase,
    state: &ProjectState,
    ahs_items: &[AhsItem],
    summary: &BoqSummary,
) -> Result<(), String> {
    if state.lines.is_empty() {
        return Err("BoQ masih kosong".to_string());
    }

    let pal = theme_palette(&state.theme);
    let fname = excel_font(&state.font);
    let eff_cells = build_effective_cells(db, state);
    let names = effective_names(db);
    let rev_names: HashMap<String, String> =
        names.iter().map(|(k, v)| (v.clone(), k.clone())).collect();
    let mut eng = FormulaEngine::new(&eff_cells, &names);

    let it_of: HashMap<&str, &AhsItem> = ahs_items.iter().map(|it| (it.k.as_str(), it)).collect();
    let block_of = |d: &str, r: u32| -> Option<&AhsItem> {
        ahs_items
            .iter()
            .find(|x| x.d == d && r >= x.start && r <= x.end)
    };

    // 1. Dependency pruning & BFS
    let mut need: HashSet<String> = HashSet::new();
    let mut qq: Vec<String> = Vec::new();
    let mut hd_need: BTreeSet<u32> = BTreeSet::new();

    for line in &state.lines {
        if !line.custom && !line.man && it_of.contains_key(line.k.as_str()) {
            if need.insert(line.k.clone()) {
                qq.push(line.k.clone());
            }
        }
    }

    while let Some(k) = qq.pop() {
        let Some(&it) = it_of.get(k.as_str()) else { continue };
        let Some(sheet_cells) = eff_cells.get(&it.d) else { continue };
        for (addr, val) in sheet_cells {
            let (_, rr) = split_addr(addr);
            if rr < it.start || rr > it.end {
                continue;
            }
            if let CellValue::Text(f) = val {
                if f.starts_with('=') {
                    for (sh_opt, row) in extract_refs(f) {
                        if sh_opt.as_deref() == Some("Harga Dasar") {
                            hd_need.insert(row);
                        } else {
                            let target_sh = sh_opt
                                .as_deref()
                                .and_then(|n| names.get(n).map(String::as_str))
                                .unwrap_or(&it.d);
                            if let Some(b) = block_of(target_sh, row) {
                                if b.k != it.k && need.insert(b.k.clone()) {
                                    qq.push(b.k.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut hq: Vec<u32> = hd_need.iter().copied().collect();
    while let Some(r) = hq.pop() {
        if let Some(CellValue::Text(f)) = eff_cells.get("hd").and_then(|m| m.get(&format!("D{}", r))) {
            if f.starts_with('=') {
                for (_, row) in extract_refs(f) {
                    if hd_need.insert(row) {
                        hq.push(row);
                    }
                }
            }
        }
    }

    let hd_rows_used: Vec<u32> = hd_need.into_iter().collect();
    let mut hd_map: HashMap<u32, u32> = HashMap::new();
    for (i, &r) in hd_rows_used.iter().enumerate() {
        hd_map.insert(r, 4 + i as u32);
    }

    // Group needed AHS items by sheet & assign compacted start rows
    let mut by_sheet: HashMap<String, Vec<&AhsItem>> = HashMap::new();
    for k in &need {
        if let Some(&it) = it_of.get(k.as_str()) {
            by_sheet.entry(it.d.clone()).or_default().push(it);
        }
    }
    for list in by_sheet.values_mut() {
        list.sort_by_key(|it| it.start);
    }

    let mut d_order: Vec<String> = by_sheet.keys().cloned().collect();
    d_order.sort();

    let mut new_starts: HashMap<String, u32> = HashMap::new();
    for d in &d_order {
        let mut row = 4u32;
        if let Some(list) = by_sheet.get(d) {
            for it in list {
                new_starts.insert(it.k.clone(), row);
                row += (it.end - it.start + 1) + 2;
            }
        }
    }

    let new_row_of = |it: &AhsItem, r: u32| -> u32 {
        new_starts
            .get(&it.k)
            .map(|&ns| ns + (r - it.start))
            .unwrap_or(r)
    };

    // Common formats
    let f_title = Format::new().set_font_name(fname).set_font_size(16).set_bold();
    let f_sub_title = Format::new().set_font_name(fname).set_font_size(13).set_bold();
    let f_bold = Format::new().set_font_name(fname).set_font_size(11).set_bold();
    let f_text = Format::new().set_font_name(fname).set_font_size(11);
    let f_wrap = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_text_wrap()
        .set_align(FormatAlign::Top);
    let f_italic_sm = Format::new()
        .set_font_name(fname)
        .set_font_size(9)
        .set_italic()
        .set_font_color(Color::RGB(0x666666));
    let f_warn_sm = Format::new()
        .set_font_name(fname)
        .set_font_size(9)
        .set_italic()
        .set_font_color(Color::RGB(0xA06000));
    let f_hdr = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(pal.acc))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_text_wrap();
    let f_dv = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(pal.dv));
    let f_dv_num = f_dv.clone().set_num_format("#,##0");
    let f_sd = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_bold()
        .set_background_color(Color::RGB(pal.soft));
    let f_sd_num = f_sd.clone().set_num_format("#,##0");
    let f_num = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_num_format("#,##0")
        .set_border_bottom(FormatBorder::Hair)
        .set_border_bottom_color(Color::RGB(0xBBBBBB));
    let f_qty = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_num_format("#,##0.00")
        .set_border_bottom(FormatBorder::Hair)
        .set_border_bottom_color(Color::RGB(0xBBBBBB));
    let f_cell_bdr = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_border_bottom(FormatBorder::Hair)
        .set_border_bottom_color(Color::RGB(0xBBBBBB));
    let f_wrap_bdr = f_wrap
        .clone()
        .set_border_bottom(FormatBorder::Hair)
        .set_border_bottom_color(Color::RGB(0xBBBBBB));
    let f_tot_num = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_bold()
        .set_num_format("#,##0");
    let f_grand_lbl = Format::new().set_font_name(fname).set_font_size(12).set_bold();
    let f_grand_num = Format::new()
        .set_font_name(fname)
        .set_font_size(12)
        .set_bold()
        .set_num_format("#,##0")
        .set_border_top(FormatBorder::Thin)
        .set_border_bottom(FormatBorder::Double);
    let f_pct = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_num_format("0.0%");

    let mut wb = Workbook::new();

    // Pre-create Rekapitulasi (index 0) and RAB (index 1)
    let _wr_idx = 0;
    let _ws_idx = 1;
    let wr = wb.add_worksheet();
    wr.set_name("Rekapitulasi").map_err(|e| e.to_string())?;
    let ws = wb.add_worksheet();
    ws.set_name("RAB").map_err(|e| e.to_string())?;

    // ===== Build RAB Sheet =====
    struct SdTrack {
        row_1b: u32,
        code: String,
        name: String,
        first_1b: Option<u32>,
        last_1b: Option<u32>,
    }
    struct DvTrack {
        row_1b: u32,
        label: String,
        key: String,
        subs: Vec<SdTrack>,
        first_1b: Option<u32>,
        last_1b: Option<u32>,
    }

    let ws = wb.worksheet_from_index(1).map_err(|e| e.to_string())?;
    ws.set_freeze_panes(9, 0).map_err(|e| e.to_string())?;
    for (c, w) in [6.0, 18.0, 56.0, 30.0, 7.0, 11.0, 17.0, 19.0].iter().enumerate() {
        ws.set_column_width(c as u16, *w).map_err(|e| e.to_string())?;
    }
    ws.merge_range(0, 0, 0, 7, "BILL OF QUANTITY", &f_title).map_err(|e| e.to_string())?;
    for (i, (lbl, val)) in doc_rows(state).iter().enumerate() {
        let rr = (1 + i) as u32;
        ws.merge_range(rr, 0, rr, 1, *lbl, &f_bold).map_err(|e| e.to_string())?;
        ws.merge_range(rr, 2, rr, 7, &format!(": {}", val), &f_text).map_err(|e| e.to_string())?;
    }
    ws.merge_range(
        6,
        0,
        6,
        7,
        &format!(
            "Dasar harga satuan: {}. Harga satuan pekerjaan AHS terhubung ke sheet AHS Div (analisa harga satuan) dan sheet Harga Dasar.",
            if state.basis == "h" { "Total incl. O&P" } else { "Bare cost" }
        ),
        &f_italic_sm,
    )
    .map_err(|e| e.to_string())?;

    let nrev = state.hd.len();
    if nrev > 0 {
        ws.merge_range(
            7,
            0,
            7,
            7,
            &format!("Harga dasar direvisi ({} item), lihat sheet Revisi Harga Dasar.", nrev),
            &f_warn_sm,
        )
        .map_err(|e| e.to_string())?;
    }

    ws.set_row_height(8, 22.0).map_err(|e| e.to_string())?;
    for (c, h) in [
        "No",
        "Kode",
        "Uraian Pekerjaan",
        "Keterangan",
        "Sat",
        "Volume",
        "Harga Satuan (Rp)",
        "Jumlah (Rp)",
    ]
    .iter()
    .enumerate()
    {
        ws.write_string_with_format(8, c as u16, *h, &f_hdr)
            .map_err(|e| e.to_string())?;
    }

    let mut r_1b = 9u32;
    let mut dvs: Vec<DvTrack> = Vec::new();
    let mut has_sd = false;

    for row in &summary.rows {
        r_1b += 1;
        let r0 = r_1b - 1;
        match row {
            BoqRowView::Division { key, label, .. } => {
                has_sd = false;
                for c in 0..7u16 {
                    ws.write_blank(r0, c, &f_dv).map_err(|e| e.to_string())?;
                }
                ws.write_string_with_format(r0, 2, label, &f_dv).map_err(|e| e.to_string())?;
                ws.write_blank(r0, 7, &f_dv_num).map_err(|e| e.to_string())?;
                dvs.push(DvTrack {
                    row_1b: r_1b,
                    label: label.clone(),
                    key: key.clone(),
                    subs: Vec::new(),
                    first_1b: None,
                    last_1b: None,
                });
            }
            BoqRowView::Subdivision { code, name, .. } => {
                has_sd = true;
                for c in 0..7u16 {
                    ws.write_blank(r0, c, &f_sd).map_err(|e| e.to_string())?;
                }
                ws.write_string_with_format(r0, 1, code, &f_sd).map_err(|e| e.to_string())?;
                ws.write_string_with_format(r0, 2, name, &f_sd).map_err(|e| e.to_string())?;
                ws.write_blank(r0, 7, &f_sd_num).map_err(|e| e.to_string())?;
                if let Some(dv) = dvs.last_mut() {
                    dv.subs.push(SdTrack {
                        row_1b: r_1b,
                        code: code.clone(),
                        name: name.clone(),
                        first_1b: None,
                        last_1b: None,
                    });
                }
            }
            BoqRowView::Line {
                no,
                k,
                u,
                s,
                qty,
                note,
                custom,
                man,
                rate,
                sum,
                ..
            } => {
                ws.write_number_with_format(r0, 0, *no as f64, &f_cell_bdr).map_err(|e| e.to_string())?;
                ws.write_string_with_format(r0, 1, if *custom { "CUSTOM" } else { k.as_str() }, &f_cell_bdr).map_err(|e| e.to_string())?;
                ws.write_string_with_format(r0, 2, u, &f_wrap_bdr).map_err(|e| e.to_string())?;
                ws.write_string_with_format(r0, 3, note, &f_wrap_bdr).map_err(|e| e.to_string())?;
                ws.write_string_with_format(r0, 4, s, &f_cell_bdr).map_err(|e| e.to_string())?;
                ws.write_number_with_format(r0, 5, *qty, &f_qty).map_err(|e| e.to_string())?;

                if !custom && !man {
                    if let Some(&it) = it_of.get(k.as_str()) {
                        let ref_addr = if state.basis == "b" { &it.cell_ref.b } else { &it.cell_ref.h };
                        let (col, row_orig) = split_addr(ref_addr);
                        let nr = new_row_of(it, row_orig);
                        let sh_title = rev_names.get(&it.d).map(String::as_str).unwrap_or("AHS");
                        let f_str = format!("='{}'!{}{}", sh_title, col, nr);
                        let f = Formula::new(&f_str).set_result(&format!("{}", rate));
                        ws.write_formula_with_format(r0, 6, f, &f_num).map_err(|e| e.to_string())?;
                    } else {
                        ws.write_number_with_format(r0, 6, (rate * 100.0).round() / 100.0, &f_num).map_err(|e| e.to_string())?;
                    }
                } else {
                    ws.write_number_with_format(r0, 6, (rate * 100.0).round() / 100.0, &f_num).map_err(|e| e.to_string())?;
                }

                let f_sum = Formula::new(&format!("=F{}*G{}", r_1b, r_1b)).set_result(&format!("{}", sum));
                ws.write_formula_with_format(r0, 7, f_sum, &f_num).map_err(|e| e.to_string())?;

                if let Some(dv) = dvs.last_mut() {
                    if has_sd {
                        if let Some(sd) = dv.subs.last_mut() {
                            if sd.first_1b.is_none() {
                                sd.first_1b = Some(r_1b);
                            }
                            sd.last_1b = Some(r_1b);
                        }
                    } else {
                        if dv.first_1b.is_none() {
                            dv.first_1b = Some(r_1b);
                        }
                        dv.last_1b = Some(r_1b);
                    }
                }
            }
        }
    }

    for dv in &dvs {
        let mut parts: Vec<String> = Vec::new();
        for sd in &dv.subs {
            if let (Some(f), Some(l)) = (sd.first_1b, sd.last_1b) {
                ws.write_formula_with_format(
                    sd.row_1b - 1,
                    7,
                    Formula::new(&format!("=SUM(H{}:H{})", f, l)),
                    &f_sd_num,
                )
                .map_err(|e| e.to_string())?;
            } else {
                ws.write_number_with_format(sd.row_1b - 1, 7, 0.0, &f_sd_num)
                    .map_err(|e| e.to_string())?;
            }
            parts.push(format!("H{}", sd.row_1b));
        }
        if let (Some(f), Some(l)) = (dv.first_1b, dv.last_1b) {
            parts.push(format!("SUM(H{}:H{})", f, l));
        }
        let dv_formula = if parts.is_empty() {
            "0".to_string()
        } else {
            format!("={}", parts.join("+"))
        };
        ws.write_formula_with_format(dv.row_1b - 1, 7, Formula::new(&dv_formula), &f_dv_num)
            .map_err(|e| e.to_string())?;
    }

    let rj = r_1b + 2;
    let rp1 = r_1b + 3;
    let rt = r_1b + 4;
    let sum_ref = dvs
        .iter()
        .map(|d| format!("H{}", d.row_1b))
        .collect::<Vec<_>>()
        .join("+");
    let sum_ref = if sum_ref.is_empty() { "0".to_string() } else { sum_ref };

    ws.write_string_with_format(rj - 1, 2, "JUMLAH", &f_bold).map_err(|e| e.to_string())?;
    ws.write_formula_with_format(
        rj - 1,
        7,
        Formula::new(&format!("={}", sum_ref)).set_result(&format!("{}", summary.subtotal)),
        &f_tot_num,
    )
    .map_err(|e| e.to_string())?;

    ws.write_string_with_format(rp1 - 1, 2, "PPN", &f_bold).map_err(|e| e.to_string())?;
    ws.write_number_with_format(rp1 - 1, 6, state.ppn / 100.0, &f_pct).map_err(|e| e.to_string())?;
    ws.write_formula_with_format(
        rp1 - 1,
        7,
        Formula::new(&format!("=H{}*G{}", rj, rp1)).set_result(&format!("{}", summary.ppn_amount)),
        &f_tot_num,
    )
    .map_err(|e| e.to_string())?;

    ws.write_string_with_format(rt - 1, 2, "TOTAL RAB", &f_grand_lbl).map_err(|e| e.to_string())?;
    ws.write_formula_with_format(
        rt - 1,
        7,
        Formula::new(&format!("=H{}+H{}", rj, rp1)).set_result(&format!("{}", summary.grand_total)),
        &f_grand_num,
    )
    .map_err(|e| e.to_string())?;

    ws.set_landscape();
    ws.set_paper_size(9);
    ws.set_print_fit_to_pages(1, 0);

    // ===== Build Rekapitulasi Sheet =====
    let wr = wb.worksheet_from_index(0).map_err(|e| e.to_string())?;
    for (c, w) in [6.0, 18.0, 62.0, 20.0, 10.0].iter().enumerate() {
        wr.set_column_width(c as u16, *w).map_err(|e| e.to_string())?;
    }
    wr.merge_range(0, 0, 0, 4, "REKAPITULASI RENCANA ANGGARAN BIAYA", &f_title)
        .map_err(|e| e.to_string())?;
    for (i, (lbl, val)) in doc_rows(state).iter().enumerate() {
        let q = (1 + i) as u32;
        wr.merge_range(q, 0, q, 1, *lbl, &f_bold).map_err(|e| e.to_string())?;
        wr.merge_range(q, 2, q, 4, &format!(": {}", val), &f_text).map_err(|e| e.to_string())?;
    }
    let rh_1b = 8u32;
    wr.set_row_height(rh_1b - 1, 22.0).map_err(|e| e.to_string())?;
    for (c, h) in ["No", "Kode", "Uraian Pekerjaan", "Jumlah (Rp)", "%"]
        .iter()
        .enumerate()
    {
        wr.write_string_with_format(rh_1b - 1, c as u16, *h, &f_hdr)
            .map_err(|e| e.to_string())?;
    }

    let mut q_1b = rh_1b;
    let mut dn = 0usize;
    let mut drows: Vec<u32> = Vec::new();
    let mut rrows: Vec<(u32, bool)> = Vec::new();

    for dv in &dvs {
        q_1b += 1;
        dn += 1;
        drows.push(q_1b);
        rrows.push((q_1b, true));
        let q0 = q_1b - 1;
        for c in 0..5u16 {
            wr.write_blank(q0, c, &f_dv).map_err(|e| e.to_string())?;
        }
        wr.write_number_with_format(q0, 0, dn as f64, &f_dv).map_err(|e| e.to_string())?;
        wr.write_string_with_format(q0, 1, if dv.key == "XX" { "" } else { &dv.key }, &f_dv)
            .map_err(|e| e.to_string())?;
        wr.write_string_with_format(q0, 2, &dv.label, &f_dv).map_err(|e| e.to_string())?;
        wr.write_formula_with_format(
            q0,
            3,
            Formula::new(&format!("='RAB'!H{}", dv.row_1b)),
            &f_dv_num,
        )
        .map_err(|e| e.to_string())?;

        let f_sd_indent = Format::new()
            .set_font_name(fname)
            .set_font_size(11)
            .set_background_color(Color::RGB(pal.soft))
            .set_indent(1);
        let f_sd_plain = Format::new()
            .set_font_name(fname)
            .set_font_size(11)
            .set_background_color(Color::RGB(pal.soft));
        let f_sd_plain_num = f_sd_plain.clone().set_num_format("#,##0");

        for sb in &dv.subs {
            q_1b += 1;
            rrows.push((q_1b, false));
            let s0 = q_1b - 1;
            for c in 0..5u16 {
                wr.write_blank(s0, c, &f_sd_plain).map_err(|e| e.to_string())?;
            }
            wr.write_string_with_format(s0, 1, &sb.code, &f_sd_plain).map_err(|e| e.to_string())?;
            wr.write_string_with_format(s0, 2, &sb.name, &f_sd_indent).map_err(|e| e.to_string())?;
            wr.write_formula_with_format(
                s0,
                3,
                Formula::new(&format!("='RAB'!H{}", sb.row_1b)),
                &f_sd_plain_num,
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let qj = q_1b + 2;
    let qp = q_1b + 3;
    let qt = q_1b + 4;
    for (row_1b, is_dv) in rrows {
        let fmt_pct = if is_dv {
            f_dv.clone().set_num_format("0.0%").set_align(FormatAlign::Right)
        } else {
            Format::new()
                .set_font_name(fname)
                .set_font_size(11)
                .set_background_color(Color::RGB(pal.soft))
                .set_num_format("0.0%")
                .set_align(FormatAlign::Right)
        };
        wr.write_formula_with_format(
            row_1b - 1,
            4,
            Formula::new(&format!("=IF($D${}=0,0,D{}/$D${})", qj, row_1b, qj)),
            &fmt_pct,
        )
        .map_err(|e| e.to_string())?;
    }

    let d_sum = drows
        .iter()
        .map(|r| format!("D{}", r))
        .collect::<Vec<_>>()
        .join("+");
    let d_sum = if d_sum.is_empty() { "0".to_string() } else { d_sum };

    wr.write_string_with_format(qj - 1, 2, "JUMLAH", &f_bold).map_err(|e| e.to_string())?;
    wr.write_formula_with_format(qj - 1, 3, Formula::new(&format!("={}", d_sum)), &f_tot_num)
        .map_err(|e| e.to_string())?;

    wr.write_string_with_format(qp - 1, 2, "PPN", &f_bold).map_err(|e| e.to_string())?;
    wr.write_formula_with_format(
        qp - 1,
        3,
        Formula::new(&format!("=D{}*'RAB'!G{}", qj, rp1)),
        &f_tot_num,
    )
    .map_err(|e| e.to_string())?;
    wr.write_formula_with_format(qp - 1, 4, Formula::new(&format!("='RAB'!G{}", rp1)), &f_pct)
        .map_err(|e| e.to_string())?;

    wr.write_string_with_format(qt - 1, 2, "TOTAL RAB", &f_grand_lbl).map_err(|e| e.to_string())?;
    wr.write_formula_with_format(
        qt - 1,
        3,
        Formula::new(&format!("=D{}+D{}", qj, qp)),
        &f_grand_num,
    )
    .map_err(|e| e.to_string())?;

    wr.set_portrait();
    wr.set_paper_size(9);
    wr.set_print_fit_to_pages(1, 0);

    // ===== Build Harga Dasar Sheet =====
    let wh = wb.add_worksheet();
    wh.set_name("Harga Dasar").map_err(|e| e.to_string())?;
    wh.set_freeze_panes(3, 0).map_err(|e| e.to_string())?;
    for (c, w) in [10.0, 62.0, 9.0, 18.0, 60.0].iter().enumerate() {
        wh.set_column_width(c as u16, *w).map_err(|e| e.to_string())?;
    }
    wh.write_string_with_format(
        0,
        0,
        "HARGA DASAR (hanya yang dipakai pada pekerjaan di RAB ini)",
        &f_sub_title,
    )
    .map_err(|e| e.to_string())?;
    for (c, h) in ["Kode", "Uraian", "Sat", "Harga / nilai", "Keterangan"]
        .iter()
        .enumerate()
    {
        wh.write_string_with_format(2, c as u16, *h, &f_hdr)
            .map_err(|e| e.to_string())?;
    }

    let hd_rows_meta = effective_hd_rows(db, state);
    let hd_meta_map: HashMap<u32, _> = hd_rows_meta.iter().map(|h| (h.r, h)).collect();
    let f_hd_num = Format::new().set_font_name(fname).set_font_size(11).set_num_format("#,##0.####");
    let f_hd_rev = f_text.clone().set_background_color(Color::RGB(0xFFF3CD));
    let f_hd_rev_num = f_hd_num.clone().set_background_color(Color::RGB(0xFFF3CD));

    for &orig_r in &hd_rows_used {
        let nr = hd_map[&orig_r];
        let r0 = nr - 1;
        let is_rev = state.hd.contains_key(&orig_r.to_string());
        let f_t = if is_rev { &f_hd_rev } else { &f_text };
        let f_n = if is_rev { &f_hd_rev_num } else { &f_hd_num };

        if let Some(meta) = hd_meta_map.get(&orig_r) {
            wh.write_string_with_format(r0, 0, &meta.k, f_t).map_err(|e| e.to_string())?;
            wh.write_string_with_format(r0, 1, &meta.n, f_t).map_err(|e| e.to_string())?;
            wh.write_string_with_format(r0, 2, &meta.u, f_t).map_err(|e| e.to_string())?;
            wh.write_string_with_format(r0, 4, &meta.c, f_t).map_err(|e| e.to_string())?;
        }

        if let Some(val) = eff_cells.get("hd").and_then(|m| m.get(&format!("D{}", orig_r))) {
            match val {
                CellValue::Number(n) => {
                    wh.write_number_with_format(r0, 3, *n, f_n).map_err(|e| e.to_string())?;
                }
                CellValue::Text(s) if s.starts_with('=') => {
                    let remapped = remap_formula_hd(s, &hd_map);
                    let res = eng.get_num("hd", &format!("D{}", orig_r)).unwrap_or(0.0);
                    let f = Formula::new(&remapped).set_result(&format!("{}", res));
                    wh.write_formula_with_format(r0, 3, f, f_n).map_err(|e| e.to_string())?;
                }
                CellValue::Text(s) => {
                    wh.write_string_with_format(r0, 3, s, f_t).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    let note_r = hd_rows_used.len() as u32 + 4;
    wh.write_string_with_format(
        note_r,
        1,
        "Baris berlatar kuning = harga dasar yang direvisi di aplikasi.",
        &f_italic_sm,
    )
    .map_err(|e| e.to_string())?;
    wh.set_portrait();
    wh.set_paper_size(9);
    wh.set_print_fit_to_pages(1, 0);

    // ===== Build AHS Div Sheets =====
    let f_ahs_cd = Format::new().set_font_name(fname).set_font_size(11).set_num_format("#,##0.####");
    let f_ahs_ef = Format::new().set_font_name(fname).set_font_size(11).set_num_format("#,##0.##");
    let f_ahs_tot_ef = f_ahs_ef.clone().set_bold().set_border_top(FormatBorder::Thin);
    let f_ahs_tot_cd = f_ahs_cd.clone().set_bold();
    let f_ahs_hdr_row = Format::new()
        .set_font_name(fname)
        .set_font_size(11)
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(pal.acc));
    let f_ahs_block_ttl = Format::new().set_font_name(fname).set_font_size(12).set_bold();

    for d in &d_order {
        let full_name = rev_names.get(d).map(String::as_str).unwrap_or(d);
        let wa = wb.add_worksheet();
        wa.set_name(&sheet_name_trunc(full_name)).map_err(|e| e.to_string())?;
        for (c, w) in [16.0, 64.0, 14.0, 14.0, 18.0, 18.0].iter().enumerate() {
            wa.set_column_width(c as u16, *w).map_err(|e| e.to_string())?;
        }
        wa.write_string_with_format(
            0,
            0,
            &format!("ANALISA HARGA SATUAN, {}", full_name.strip_prefix("AHS ").unwrap_or(full_name)),
            &f_sub_title,
        )
        .map_err(|e| e.to_string())?;

        let Some(sheet_cells) = eff_cells.get(d) else { continue };
        let Some(list) = by_sheet.get(d) else { continue };

        for it in list {
            for r in it.start..=it.end {
                let nr = new_row_of(it, r);
                let r0 = nr - 1;

                let cell_str = |col: &str| -> String {
                    match sheet_cells.get(&format!("{}{}", col, r)) {
                        Some(CellValue::Text(s)) => s.clone(),
                        _ => String::new(),
                    }
                };
                let a_txt = cell_str("A");
                let b_txt = cell_str("B");

                let is_sec = a_txt.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                    && a_txt.contains(". ");
                let is_tbl_hdr = a_txt == "Kode" || b_txt == "Uraian" || b_txt == "Komponen";
                let is_tot = b_txt.starts_with("Jumlah")
                    || b_txt.starts_with("Total")
                    || b_txt.starts_with("TOTAL")
                    || b_txt.starts_with("Dibulatkan");

                if is_sec {
                    for c in 0..6u16 {
                        wa.write_blank(r0, c, &f_sd).map_err(|e| e.to_string())?;
                    }
                } else if is_tbl_hdr {
                    for c in 0..6u16 {
                        wa.write_blank(r0, c, &f_ahs_hdr_row).map_err(|e| e.to_string())?;
                    }
                }

                for (c_idx, col) in ["A", "B", "C", "D", "E", "F"].iter().enumerate() {
                    let addr = format!("{}{}", col, r);
                    let Some(val) = sheet_cells.get(&addr) else { continue };

                    let fmt = if r == it.start && c_idx == 0 {
                        &f_ahs_block_ttl
                    } else if r <= it.start + 2 && c_idx == 0 {
                        &f_italic_sm
                    } else if is_sec {
                        &f_sd
                    } else if is_tbl_hdr {
                        &f_ahs_hdr_row
                    } else if is_tot && c_idx >= 1 {
                        if c_idx == 5 {
                            &f_ahs_tot_ef
                        } else if c_idx == 2 || c_idx == 3 {
                            &f_ahs_tot_cd
                        } else {
                            &f_bold
                        }
                    } else if c_idx == 2 || c_idx == 3 {
                        &f_ahs_cd
                    } else if c_idx == 4 || c_idx == 5 {
                        &f_ahs_ef
                    } else {
                        &f_text
                    };

                    match val {
                        CellValue::Number(n) => {
                            wa.write_number_with_format(r0, c_idx as u16, *n, fmt)
                                .map_err(|e| e.to_string())?;
                        }
                        CellValue::Text(s) if s.starts_with('=') => {
                            let remapped =
                                remap_formula_ahs(s, d, &names, &hd_map, ahs_items, &new_starts);
                            let mut f = Formula::new(&remapped);
                            if let Ok(EvalValue::Number(res)) = eng.get(d, &addr) {
                                f = f.set_result(&format!("{}", res));
                            }
                            wa.write_formula_with_format(r0, c_idx as u16, f, fmt)
                                .map_err(|e| e.to_string())?;
                        }
                        CellValue::Text(s) => {
                            wa.write_string_with_format(r0, c_idx as u16, s, fmt)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }
        wa.set_portrait();
        wa.set_paper_size(9);
        wa.set_print_fit_to_pages(1, 0);
    }

    // ===== Build Revisi Harga Dasar Sheet (if any) =====
    if !state.hd.is_empty() {
        let w2 = wb.add_worksheet();
        w2.set_name("Revisi Harga Dasar").map_err(|e| e.to_string())?;
        for (c, w) in [10.0, 62.0, 10.0, 16.0, 16.0].iter().enumerate() {
            w2.set_column_width(c as u16, *w).map_err(|e| e.to_string())?;
        }
        w2.write_string_with_format(0, 0, "Harga dasar yang direvisi", &f_sub_title)
            .map_err(|e| e.to_string())?;
        for (c, h) in ["Kode", "Uraian", "Sat", "Harga awal", "Harga revisi"]
            .iter()
            .enumerate()
        {
            w2.write_string_with_format(2, c as u16, *h, &f_bold)
                .map_err(|e| e.to_string())?;
        }
        let mut r0 = 3u32;
        for row in &hd_rows_meta {
            if let Some(&rev_val) = state.hd.get(&row.r.to_string()) {
                let orig = db
                    .payload
                    .cells
                    .get("hd")
                    .and_then(|m| m.get(&format!("D{}", row.r)))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                w2.write_string_with_format(r0, 0, &row.k, &f_text).map_err(|e| e.to_string())?;
                w2.write_string_with_format(r0, 1, &row.n, &f_text).map_err(|e| e.to_string())?;
                w2.write_string_with_format(r0, 2, &row.u, &f_text).map_err(|e| e.to_string())?;
                w2.write_number_with_format(r0, 3, orig, &f_hd_num).map_err(|e| e.to_string())?;
                w2.write_number_with_format(r0, 4, rev_val, &f_hd_num).map_err(|e| e.to_string())?;
                r0 += 1;
            }
        }
    }

    wb.save(path).map_err(|e| format!("Gagal menyimpan Excel: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ahs_builder::build_all_ahs_items;
    use crate::domain::boq::{build_all_catalog, build_boq_summary, effective_divs};
    use crate::storage::autosave::seed_example_lines;

    #[test]
    fn exports_valid_xlsx_with_example_boq() {
        let db = StaticDatabase::load_embedded().unwrap();
        let mut state = ProjectState::default();
        seed_example_lines(&db, &mut state);
        state.hd.insert("4".to_string(), 145000.0);

        let ahs = build_all_ahs_items(&db, &state).unwrap();
        let cat = build_all_catalog(&db, &ahs);
        let divs = effective_divs(&db, &state);
        let summary = build_boq_summary(&state, &ahs, &cat, &divs);

        let tmp = std::env::temp_dir().join("rabbro_test_export.xlsx");
        export_xlsx(&tmp, &db, &state, &ahs, &summary).expect("Excel export failed");
        let meta = std::fs::metadata(&tmp).unwrap();
        assert!(meta.len() > 2000);
        let _ = std::fs::remove_file(&tmp);
    }
}
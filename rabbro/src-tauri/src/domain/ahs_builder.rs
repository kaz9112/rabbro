use std::collections::{HashMap, HashSet};

use crate::engine::recalc::{build_effective_cells, effective_names, recalc_items};
use crate::models::catalog::{AhsItem, AhsRef, CellValue, HdRow, StaticDatabase};
use crate::models::state::{ProjectState, UserAhs, UserAhsSpec};

pub const STRIDE: u32 = 300;
pub const UNAME: &str = "AHS Buatan Pengguna";

pub fn effective_hd_rows(db: &StaticDatabase, state: &ProjectState) -> Vec<HdRow> {
    let mut rows = db.payload.hd.clone();
    for u in &state.uhd {
        rows.push(HdRow {
            r: u.r,
            k: u.k.clone(),
            n: u.n.clone(),
            u: u.u.clone(),
            c: u.c.clone(),
            f: false,
            usr: true,
        });
    }
    rows
}

pub fn build_all_ahs_items(
    db: &StaticDatabase,
    state: &ProjectState,
) -> Result<Vec<AhsItem>, String> {
    let mut items = db.payload.items.clone();

    let mut user_sorted = state.uahs.clone();
    user_sorted.sort_by(|a, b| a.dv.cmp(&b.dv).then_with(|| a.k.cmp(&b.k)));

    for ua in user_sorted {
        items.push(AhsItem {
            k: ua.k,
            u: ua.u,
            s: ua.s,
            g: ua.g,
            dv: ua.dv,
            d: "usr".to_string(),
            cell_ref: ua.cell_ref,
            h0: 0.0,
            b0: 0.0,
            start: ua.start,
            end: ua.end,
            usr: Some(ua.id),
            from: ua.from,
            spec: ua.spec.is_some(),
            h: 0.0,
            b: 0.0,
        });
    }

    let names = effective_names(db);

    // Compute baseline h0/b0 for user-created AHS (without hd/ov overrides)
    let mut base_state = ProjectState::default();
    base_state.uahs = state.uahs.clone();
    base_state.uhd = state.uhd.clone();
    let base_cells = build_effective_cells(db, &base_state);
    recalc_items(&mut items, &base_cells, &names, None)?;
    for it in &mut items {
        if it.usr.is_some() {
            it.h0 = it.h;
            it.b0 = it.b;
        }
    }

    // Compute current h/b with all overrides applied
    let eff_cells = build_effective_cells(db, state);
    recalc_items(&mut items, &eff_cells, &names, None)?;

    Ok(items)
}

pub fn next_derived_code(src: &AhsItem, existing_codes: &HashSet<String>) -> String {
    let sep = if src.from.is_empty() { "-R" } else { "." };
    let mut n = 1u32;
    loop {
        let candidate = format!("{}{}{}", src.k, sep, n);
        if !existing_codes.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

pub fn hdr_line(dv: &str, g: &str, s: &str) -> String {
    format!("Divisi {}  |  Subdivisi: {}  |  Satuan: {}", dv, g, s)
}

fn shift_addr(addr: &str, off: i32) -> String {
    let idx = addr.find(|c: char| c.is_ascii_digit()).unwrap_or(1);
    let col = &addr[..idx];
    let row: i32 = addr[idx..].parse().unwrap_or(0);
    format!("{}{}", col, row + off)
}

/// Shifts cell references inside [s0, s1] by `off`, and qualifies external references with sheet name.
pub fn move_formula(
    formula: &str,
    src_sh: &str,
    s0: u32,
    s1: u32,
    off: i32,
    names: &HashMap<String, String>,
) -> String {
    let rev_names: HashMap<&str, &str> =
        names.iter().map(|(k, v)| (v.as_str(), k.as_str())).collect();

    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::with_capacity(formula.len() + 16);
    let mut i = 0;

    while i < chars.len() {
        // Check for optional quoted sheet prefix: 'Sheet Name'!
        let mut sh_name: Option<String> = None;
        let saved_i = i;

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

        // Try to parse cell ref or range: ($?)([A-Z]{1,2})($?)(\d+)
        if let Some((c1_d, col1, r1_d, row1, next_i)) = parse_cell_token(&chars, i) {
            let sk = sh_name
                .as_deref()
                .and_then(|n| names.get(n).map(String::as_str))
                .unwrap_or(src_sh);

            // Check if followed by :($?)([A-Z]{1,2})($?)(\d+)
            if next_i < chars.len() && chars[next_i] == ':' {
                if let Some((c2_d, col2, r2_d, row2, end_i)) = parse_cell_token(&chars, next_i + 1) {
                    let in_block = sk == src_sh
                        && row1 >= s0
                        && row1 <= s1
                        && row2 >= s0
                        && row2 <= s1;
                    if in_block {
                        out.push_str(&format!(
                            "{}{}{}{}:{}{}{}{}",
                            c1_d,
                            col1,
                            r1_d,
                            (row1 as i32 + off),
                            c2_d,
                            col2,
                            r2_d,
                            (row2 as i32 + off)
                        ));
                    } else {
                        let sheet_title = rev_names.get(sk).copied().unwrap_or("Harga Dasar");
                        out.push_str(&format!(
                            "'{}'!{}{}{}{}:{}{}{}{}",
                            sheet_title, c1_d, col1, r1_d, row1, c2_d, col2, r2_d, row2
                        ));
                    }
                    i = end_i;
                    continue;
                }
            }

            // Single cell reference
            let in_block = sk == src_sh && row1 >= s0 && row1 <= s1;
            if in_block {
                out.push_str(&format!(
                    "{}{}{}{}",
                    c1_d,
                    col1,
                    r1_d,
                    (row1 as i32 + off)
                ));
            } else {
                let sheet_title = rev_names.get(sk).copied().unwrap_or("Harga Dasar");
                out.push_str(&format!(
                    "'{}'!{}{}{}{}",
                    sheet_title, c1_d, col1, r1_d, row1
                ));
            }
            i = next_i;
            continue;
        }

        // Not a cell reference: restore and emit single char
        if sh_name.is_some() {
            i = saved_i;
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

fn parse_cell_token(chars: &[char], mut i: usize) -> Option<(String, String, String, u32, usize)> {
    // Must not be preceded by alphanumeric (e.g. inside identifier)
    let c_dollar = if i < chars.len() && chars[i] == '$' {
        i += 1;
        "$"
    } else {
        ""
    };
    let col_start = i;
    while i < chars.len() && chars[i].is_ascii_uppercase() && (i - col_start) < 2 {
        i += 1;
    }
    if i == col_start {
        return None;
    }
    let col: String = chars[col_start..i].iter().collect();

    let r_dollar = if i < chars.len() && chars[i] == '$' {
        i += 1;
        "$"
    } else {
        ""
    };
    let row_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == row_start {
        return None;
    }
    // If followed immediately by '(' or alphabetic char, it's a function name or invalid token
    if i < chars.len() && (chars[i] == '(' || chars[i].is_ascii_alphabetic()) {
        return None;
    }
    let row_str: String = chars[row_start..i].iter().collect();
    let row: u32 = row_str.parse().ok()?;
    Some((
        c_dollar.to_string(),
        col,
        r_dollar.to_string(),
        row,
        i,
    ))
}

pub fn derive_user_ahs(
    id: u32,
    src: &AhsItem,
    new_code: String,
    u: String,
    s: String,
    dv: String,
    g: String,
    note: &str,
    effective_cells: &HashMap<String, HashMap<String, CellValue>>,
    names: &HashMap<String, String>,
) -> Result<UserAhs, String> {
    let start = 5 + id * STRIDE;
    let off = start as i32 - src.start as i32;
    let src_cells = effective_cells
        .get(&src.d)
        .ok_or_else(|| format!("Sheet sumber '{}' tidak ditemukan", src.d))?;

    let mut cells = HashMap::new();
    for (addr, val) in src_cells {
        let row_num: u32 = addr
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        if row_num < src.start || row_num > src.end {
            continue;
        }
        let col_part: String = addr.chars().take_while(|c| !c.is_ascii_digit()).collect();
        let new_addr = format!("{}{}", col_part, (row_num as i32 + off));
        let new_val = match val {
            CellValue::Text(t) if t.starts_with('=') => {
                CellValue::Text(move_formula(t, &src.d, src.start, src.end, off, names))
            }
            other => other.clone(),
        };
        cells.insert(new_addr, new_val);
    }

    cells.insert(
        format!("A{}", start),
        CellValue::Text(format!("{}  {}", new_code, u)),
    );
    let note_suffix = if note.trim().is_empty() {
        String::new()
    } else {
        format!(". Catatan: {}", note.trim())
    };
    let src_desc: String = src.u.chars().take(100).collect();
    cells.insert(
        format!("A{}", start + 1),
        CellValue::Text(format!("Turunan dari {}  {}{}", src.k, src_desc, note_suffix)),
    );
    cells.insert(
        format!("A{}", start + 2),
        CellValue::Text(hdr_line(&dv, &g, &s)),
    );

    Ok(UserAhs {
        id,
        k: new_code,
        u,
        s,
        g,
        dv,
        start,
        end: (src.end as i32 + off) as u32,
        cell_ref: AhsRef {
            j: shift_addr(&src.cell_ref.j, off),
            b: shift_addr(&src.cell_ref.b, off),
            h: shift_addr(&src.cell_ref.h, off),
        },
        cells,
        from: src.k.clone(),
        spec: None,
    })
}

pub fn resolve_hd<'a>(text: &str, cat: Option<char>, hd_rows: &'a [HdRow]) -> Option<&'a HdRow> {
    let t = text.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }
    let code_part = t.split(|c: char| c.is_whitespace() || c == '|').next().unwrap_or("");
    let found = hd_rows.iter().find(|x| {
        let lbl = format!("{} | {}", x.k, x.n).to_ascii_lowercase();
        lbl == t || x.k.to_ascii_lowercase() == code_part
    })?;
    match cat {
        Some(c) if found.k.starts_with(c) => Some(found),
        None if matches!(found.k.chars().next(), Some('L' | 'M' | 'E')) => Some(found),
        _ => None,
    }
}

pub fn gen_spec_user_ahs(
    id: u32,
    k: String,
    u: String,
    s: String,
    dv: String,
    g: String,
    spec: &UserAhsSpec,
    hd_rows: &[HdRow],
    existing_start: Option<u32>,
) -> Result<UserAhs, String> {
    let start = existing_start.unwrap_or(5 + id * STRIDE);
    let mut c: HashMap<String, CellValue> = HashMap::new();
    let mut r = start;
    let hdq = "'Harga Dasar'!";
    let txt = |v: String| CellValue::Text(v);
    let num = |v: f64| CellValue::Number(v);

    c.insert(format!("A{}", r), txt(format!("{}  {}", k, u)));
    r += 1;
    c.insert(format!("A{}", r), txt(format!("Kode AHS buatan pengguna: {}", k)));
    r += 1;
    c.insert(format!("A{}", r), txt(hdr_line(&dv, &g, &s)));
    r += 2;

    c.insert(format!("A{}", r), txt("1. Crew (regu kerja)".into()));
    r += 1;
    for (idx, title) in [
        "Kode",
        "Anggota crew",
        "Jumlah (orang)",
        "Upah (Rp/OH)",
        "Biaya per hari (Rp)",
        "Jam-orang per hari",
    ]
    .iter()
    .enumerate()
    {
        let col = (b'A' + idx as u8) as char;
        c.insert(format!("{}{}", col, r), txt((*title).into()));
    }
    r += 1;

    let c1 = r;
    for (i, cr) in spec.crew.iter().enumerate() {
        let hd = resolve_hd(&cr.t, Some('L'), hd_rows)
            .ok_or_else(|| format!("Crew baris {}: pilih upah dari daftar", i + 1))?;
        if !(cr.n > 0.0) {
            return Err(format!("Crew baris {}: jumlah orang harus > 0", i + 1));
        }
        c.insert(format!("A{}", r), txt(hd.k.clone()));
        c.insert(format!("B{}", r), txt(hd.n.clone()));
        c.insert(format!("C{}", r), num(cr.n));
        c.insert(format!("D{}", r), txt(format!("={}D{}", hdq, hd.r)));
        c.insert(format!("E{}", r), txt(format!("=C{}*D{}", r, r)));
        c.insert(format!("F{}", r), txt(format!("=C{}*{}$D$27", r, hdq)));
        r += 1;
    }
    let c2 = r - 1;
    let ct = r;
    c.insert(format!("B{}", ct), txt("Jumlah crew".into()));
    if !spec.crew.is_empty() {
        for col in ['C', 'E', 'F'] {
            c.insert(
                format!("{}{}", col, ct),
                txt(format!("=SUM({}{}:{}{})", col, c1, col, c2)),
            );
        }
    } else {
        for col in ['C', 'E', 'F'] {
            c.insert(format!("{}{}", col, ct), num(0.0));
        }
    }
    r += 2;

    c.insert(format!("A{}", r), txt("2. Produktivitas".into()));
    r += 1;
    c.insert(format!("B{}", r), txt("Uraian".into()));
    c.insert(format!("C{}", r), txt("Satuan".into()));
    c.insert(format!("D{}", r), txt("Nilai".into()));
    r += 1;
    let ro = r;
    c.insert(format!("B{}", r), txt("Daily output".into()));
    c.insert(format!("C{}", r), txt(format!("{}/hari", s)));
    c.insert(format!("D{}", r), num(spec.out));
    r += 1;
    let rj = r;
    c.insert(format!("B{}", r), txt(format!("Labor-hours per {}", s)));
    c.insert(format!("C{}", r), txt(format!("jam/{}", s)));
    c.insert(format!("D{}", r), txt(format!("=F{}/D{}", ct, ro)));
    r += 2;

    c.insert(
        format!("A{}", r),
        txt(format!("3. Biaya langsung per {} (bare cost)", s)),
    );
    r += 1;
    for (idx, title) in [
        "Kode",
        "Uraian",
        "Satuan",
        "Koefisien",
        "Harga Satuan (Rp)",
        "Jumlah (Rp)",
    ]
    .iter()
    .enumerate()
    {
        let col = (b'A' + idx as u8) as char;
        c.insert(format!("{}{}", col, r), txt((*title).into()));
    }
    r += 1;

    let m1 = r;
    for mat in &spec.mats {
        let hd = resolve_hd(&mat.t, Some('M'), hd_rows);
        let name = hd.map(|h| h.n.as_str()).unwrap_or(mat.t.trim());
        let unit = if mat.u.trim().is_empty() {
            hd.map(|h| h.u.as_str()).unwrap_or(&s)
        } else {
            mat.u.trim()
        };
        c.insert(
            format!("A{}", r),
            txt(hd.map(|h| h.k.clone()).unwrap_or_default()),
        );
        c.insert(format!("B{}", r), txt(format!("Material: {}", name)));
        c.insert(format!("C{}", r), txt(unit.into()));
        c.insert(format!("D{}", r), num(mat.c));
        c.insert(
            format!("E{}", r),
            if let Some(h) = hd {
                txt(format!("={}D{}", hdq, h.r))
            } else {
                num(mat.p)
            },
        );
        c.insert(format!("F{}", r), txt(format!("=D{}*E{}", r, r)));
        r += 1;
    }
    let m2 = r - 1;
    let rl = r;
    c.insert(
        format!("B{}", r),
        txt("Labor: biaya crew per hari / daily output".into()),
    );
    c.insert(format!("C{}", r), txt(s.clone()));
    c.insert(format!("D{}", r), num(1.0));
    c.insert(
        format!("E{}", r),
        if spec.crew.is_empty() {
            num(0.0)
        } else {
            txt(format!("=E{}/D{}", ct, ro))
        },
    );
    c.insert(format!("F{}", r), txt(format!("=D{}*E{}", r, r)));
    r += 1;

    let e1 = r;
    for eq in &spec.eqs {
        let hd = resolve_hd(&eq.t, Some('E'), hd_rows);
        let name = hd.map(|h| h.n.as_str()).unwrap_or(eq.t.trim());
        let unit = if eq.u.trim().is_empty() {
            hd.map(|h| h.u.as_str()).unwrap_or(&s)
        } else {
            eq.u.trim()
        };
        c.insert(
            format!("A{}", r),
            txt(hd.map(|h| h.k.clone()).unwrap_or_default()),
        );
        c.insert(format!("B{}", r), txt(format!("Equipment: {}", name)));
        c.insert(format!("C{}", r), txt(unit.into()));
        c.insert(format!("D{}", r), num(eq.c));
        c.insert(
            format!("E{}", r),
            if let Some(h) = hd {
                txt(format!("={}D{}", hdq, h.r))
            } else {
                num(eq.p)
            },
        );
        c.insert(format!("F{}", r), txt(format!("=D{}*E{}", r, r)));
        r += 1;
    }
    let e2 = r - 1;
    let rb = r;
    c.insert(
        format!("B{}", r),
        txt("Total bare cost (Material + Labor + Equipment)".into()),
    );
    c.insert(format!("F{}", r), txt(format!("=SUM(F{}:F{})", m1, r - 1)));
    r += 2;

    c.insert(
        format!("A{}", r),
        txt("4. Total termasuk Overhead & Profit (O&P)".into()),
    );
    r += 1;
    c.insert(format!("B{}", r), txt("Komponen".into()));
    c.insert(format!("D{}", r), txt("Markup O&P".into()));
    c.insert(format!("E{}", r), txt("Bare cost (Rp)".into()));
    c.insert(format!("F{}", r), txt("Total incl. O&P (Rp)".into()));
    r += 1;

    let o1 = r;
    c.insert(format!("B{}", r), txt("Material".into()));
    c.insert(format!("D{}", r), txt(format!("={}D15", hdq)));
    c.insert(
        format!("E{}", r),
        if spec.mats.is_empty() {
            num(0.0)
        } else {
            txt(format!("=SUM(F{}:F{})", m1, m2))
        },
    );
    c.insert(format!("F{}", r), txt(format!("=E{}*(1+D{})", r, r)));
    r += 1;

    c.insert(format!("B{}", r), txt("Labor".into()));
    c.insert(format!("D{}", r), txt(format!("={}D25", hdq)));
    c.insert(format!("E{}", r), txt(format!("=F{}", rl)));
    c.insert(format!("F{}", r), txt(format!("=E{}*(1+D{})", r, r)));
    r += 1;

    c.insert(format!("B{}", r), txt("Equipment".into()));
    c.insert(format!("D{}", r), txt(format!("={}D26", hdq)));
    c.insert(
        format!("E{}", r),
        if spec.eqs.is_empty() {
            num(0.0)
        } else {
            txt(format!("=SUM(F{}:F{})", e1, e2))
        },
    );
    c.insert(format!("F{}", r), txt(format!("=E{}*(1+D{})", r, r)));
    r += 1;

    let o2 = r - 1;
    let rt = r;
    c.insert(format!("B{}", r), txt(format!("TOTAL INCL. O&P per {}", s)));
    c.insert(format!("E{}", r), txt(format!("=SUM(E{}:E{})", o1, o2)));
    c.insert(format!("F{}", r), txt(format!("=SUM(F{}:F{})", o1, o2)));
    r += 1;

    c.insert(format!("B{}", r), txt("Dibulatkan".into()));
    c.insert(format!("F{}", r), txt(format!("=ROUND(F{},-2)", rt)));

    Ok(UserAhs {
        id,
        k,
        u,
        s,
        g,
        dv,
        start,
        end: r,
        cell_ref: AhsRef {
            j: format!("D{}", rj),
            b: format!("F{}", rb),
            h: format!("F{}", r),
        },
        cells: c,
        from: String::new(),
        spec: Some(spec.clone()),
    })
}

pub fn hd_is_used(row_num: u32, state: &ProjectState) -> bool {
    let target_plain = format!("'Harga Dasar'!D{}", row_num);
    let target_abs = format!("'Harga Dasar'!$D${}", row_num);
    let check = |v: &CellValue| -> bool {
        if let CellValue::Text(s) = v {
            for pat in [&target_plain, &target_abs] {
                if let Some(pos) = s.find(pat.as_str()) {
                    let after = &s[pos + pat.len()..];
                    if !after.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        return true;
                    }
                }
            }
        }
        false
    };
    state
        .uahs
        .iter()
        .any(|ua| ua.cells.values().any(check))
        || state.ov.values().any(check)
}
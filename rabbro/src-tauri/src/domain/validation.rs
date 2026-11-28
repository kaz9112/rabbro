use std::collections::{HashMap, HashSet};

use super::ahs_builder::STRIDE;
use super::boq::norm_g;
use crate::engine::parser::parse_user_input;
use crate::engine::recalc::effective_names;
use crate::models::catalog::{CellValue, StaticDatabase};
use crate::models::state::{BoqLine, ProjectState, UserAhs, UserHdItem};

fn trunc(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

fn valid_cell_addr(a: &str) -> bool {
    let idx = match a.find(|c: char| c.is_ascii_digit()) {
        Some(i) => i,
        None => return false,
    };
    let (col, row_s) = a.split_at(idx);
    (1..=2).contains(&col.len())
        && col.chars().all(|c| c.is_ascii_uppercase())
        && !row_s.is_empty()
        && row_s.chars().all(|c| c.is_ascii_digit())
}

fn valid_hd_code(k: &str) -> bool {
    let bytes = k.as_bytes();
    bytes.len() >= 3
        && bytes.len() <= 10
        && matches!(bytes[0], b'L' | b'M' | b'E')
        && bytes[1] == b'.'
        && bytes[2..].iter().all(|b| b.is_ascii_alphanumeric())
}

pub fn sanitize_state(raw: ProjectState, db: &StaticDatabase) -> (ProjectState, usize) {
    let mut skipped = 0usize;
    let names = effective_names(db);

    // 1. User divisions
    let mut udivs: HashMap<String, String> = HashMap::new();
    for (c, nm) in raw.udivs {
        let valid_code = (1..=4).contains(&c.len()) && c.chars().all(|ch| ch.is_ascii_alphanumeric());
        let name = trunc(&nm, 80);
        if valid_code && !db.payload.divs.contains_key(&c) && !name.is_empty() {
            udivs.insert(c, name);
        }
    }

    // 2. User AHS items
    let base_ahs_codes: HashSet<&str> = db.payload.items.iter().map(|i| i.k.as_str()).collect();
    let mut uahs: Vec<UserAhs> = Vec::new();
    let mut seen_u_codes: HashSet<String> = HashSet::new();
    let mut seen_starts: HashSet<u32> = HashSet::new();
    let mut unext = 0u32;

    for mut ua in raw.uahs {
        ua.k = trunc(ua.k.trim(), 40);
        ua.u = trunc(&ua.u, 300);
        ua.s = trunc(&ua.s, 20);
        ua.g = norm_g(&trunc(&ua.g, 120));
        ua.from = trunc(&ua.from, 60);

        let dv_ok = db.payload.divs.contains_key(&ua.dv) || udivs.contains_key(&ua.dv);
        let start_ok = ua.start >= 5
            && (ua.start - 5) % STRIDE == 0
            && (ua.start - 5) / STRIDE <= 5000
            && ua.end >= ua.start
            && ua.end - ua.start < STRIDE;

        if ua.k.is_empty()
            || ua.u.is_empty()
            || ua.s.is_empty()
            || ua.g.is_empty()
            || !dv_ok
            || !start_ok
            || base_ahs_codes.contains(ua.k.as_str())
            || seen_u_codes.contains(&ua.k)
            || seen_starts.contains(&ua.start)
        {
            skipped += 1;
            continue;
        }

        let in_range = |a: &str| -> bool {
            if !valid_cell_addr(a) {
                return false;
            }
            let r: u32 = a
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0);
            r >= ua.start && r <= ua.end
        };

        if !in_range(&ua.cell_ref.j) || !in_range(&ua.cell_ref.b) || !in_range(&ua.cell_ref.h) {
            skipped += 1;
            continue;
        }

        let mut cells_ok = true;
        for (addr, val) in &ua.cells {
            if !in_range(addr) {
                cells_ok = false;
                break;
            }
            match val {
                CellValue::Number(n) if !n.is_finite() => {
                    cells_ok = false;
                    break;
                }
                CellValue::Text(t) if t.len() > 600 => {
                    cells_ok = false;
                    break;
                }
                CellValue::Text(t) if t.starts_with('=') && parse_user_input(t, &names).is_err() => {
                    cells_ok = false;
                    break;
                }
                _ => {}
            }
        }
        if !cells_ok {
            skipped += 1;
            continue;
        }

        ua.id = (ua.start - 5) / STRIDE;
        unext = unext.max(ua.id + 1);
        seen_u_codes.insert(ua.k.clone());
        seen_starts.insert(ua.start);
        uahs.push(ua);
    }

    // 3. User Harga Dasar items
    let base_hd_codes: HashSet<String> = db
        .payload
        .hd
        .iter()
        .map(|h| h.k.to_ascii_lowercase())
        .collect();
    let mut uhd: Vec<UserHdItem> = Vec::new();
    let mut seen_hd_codes: HashSet<String> = HashSet::new();
    let mut seen_hd_ids: HashSet<u32> = HashSet::new();
    let mut uhnext = 0u32;

    for mut item in raw.uhd {
        item.k = trunc(item.k.trim(), 12);
        item.n = trunc(item.n.trim(), 160);
        item.u = trunc(item.u.trim(), 12);
        item.c = trunc(&item.c, 160);
        let lk = item.k.to_ascii_lowercase();

        if item.id > 99999
            || !valid_hd_code(&item.k)
            || item.n.is_empty()
            || item.u.is_empty()
            || !item.v.is_finite()
            || item.v < 0.0
            || base_hd_codes.contains(&lk)
            || seen_hd_codes.contains(&lk)
            || seen_hd_ids.contains(&item.id)
        {
            skipped += 1;
            continue;
        }
        item.r = 100000 + item.id;
        uhnext = uhnext.max(item.id + 1);
        seen_hd_codes.insert(lk);
        seen_hd_ids.insert(item.id);
        uhd.push(item);
    }

    // 4. BoQ lines
    let man_map: HashMap<&str, &str> = db
        .manual
        .man
        .iter()
        .chain(db.manual.mep.iter())
        .map(|m| (m.k.as_str(), m.u.as_str()))
        .collect();
    let ahs_map: HashMap<&str, (&str, &str)> = db
        .payload
        .items
        .iter()
        .map(|it| (it.k.as_str(), (it.u.as_str(), it.s.as_str())))
        .collect();
    let uahs_map: HashMap<&str, (&str, &str)> = uahs
        .iter()
        .map(|ua| (ua.k.as_str(), (ua.u.as_str(), ua.s.as_str())))
        .collect();

    let mut lines: Vec<BoqLine> = Vec::new();
    for mut line in raw.lines {
        line.u = trunc(&line.u, 300);
        line.s = trunc(&line.s, 20);
        line.note = trunc(&line.note, 300);
        if !line.qty.is_finite() {
            line.qty = 0.0;
        }

        if line.custom {
            line.k.clear();
            if !line.price.is_finite() {
                line.price = 0.0;
            }
            if !db.payload.divs.contains_key(&line.dv) && !udivs.contains_key(&line.dv) {
                line.dv.clear();
            }
            line.g = if line.dv.is_empty() {
                String::new()
            } else {
                norm_g(&trunc(&line.g, 120))
            };
        } else if line.man {
            if let Some(&u) = man_map.get(line.k.as_str()) {
                line.u = u.to_string();
                if !line.price.is_finite() {
                    line.price = 0.0;
                }
            } else {
                skipped += 1;
                continue;
            }
        } else if let Some(&(u, s)) = ahs_map
            .get(line.k.as_str())
            .or_else(|| uahs_map.get(line.k.as_str()))
        {
            line.u = u.to_string();
            line.s = s.to_string();
        } else {
            skipped += 1;
            continue;
        }

        line.id = (lines.len() + 1) as u32;
        lines.push(line);
    }

    // 5. HD overrides
    let base_hd_rows: HashSet<u32> = db.payload.hd.iter().map(|h| h.r).collect();
    let uhd_rows: HashSet<u32> = uhd.iter().map(|h| h.r).collect();
    let mut hd: HashMap<String, f64> = HashMap::new();
    for (r_str, v) in raw.hd {
        if let Ok(r_num) = r_str.parse::<u32>() {
            if (base_hd_rows.contains(&r_num) || uhd_rows.contains(&r_num)) && v.is_finite() {
                hd.insert(r_str, v);
            }
        }
    }

    // 6. AHS overrides
    let mut ov: HashMap<String, CellValue> = HashMap::new();
    for (key, val) in raw.ov {
        let Some((sh, addr)) = key.split_once(':') else {
            continue;
        };
        let sh_ok = sh == "usr" || db.payload.cells.contains_key(sh);
        if !sh_ok || !valid_cell_addr(addr) {
            continue;
        }
        match val {
            CellValue::Number(n) if n.is_finite() => {
                ov.insert(key, CellValue::Number(n));
            }
            CellValue::Text(t) => {
                if let Ok(crate::engine::parser::ParsedInput::Formula(f)) =
                    parse_user_input(&t, &names)
                {
                    ov.insert(key, CellValue::Text(f));
                }
            }
            _ => {}
        }
    }

    let title = {
        let t = trunc(&raw.title, 120);
        if t.is_empty() || t == "Rencana Anggaran Biaya" {
            "RaBro".to_string()
        } else {
            t
        }
    };

    let clean = ProjectState {
        fname: trunc(&raw.fname, 150),
        title,
        sub: trunc(&raw.sub, 200),
        proj: trunc(&raw.proj, 150),
        company: trunc(&raw.company, 150),
        by: trunc(&raw.by, 100),
        date: trunc(&raw.date, 10),
        rev: trunc(&raw.rev, 10),
        basis: if raw.basis == "b" { "b".into() } else { "h".into() },
        ppn: if raw.ppn.is_finite() { raw.ppn } else { 11.0 },
        lines,
        hd,
        ov,
        uahs,
        udivs,
        unext,
        uhd,
        uhnext,
        theme: if raw.theme.is_empty() { "exist".into() } else { raw.theme },
        font: if raw.font.is_empty() { "exist".into() } else { raw.font },
    };

    (clean, skipped)
}
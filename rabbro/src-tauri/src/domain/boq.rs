use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::catalog::{AhsItem, StaticDatabase};
use crate::models::state::{BoqLine, ProjectState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub k: String,
    pub u: String,
    pub s: String,
    pub g: String,
    pub dv: String,
    pub m: bool,
    pub usr: bool,
    pub h: f64,
    pub b: f64,
    pub h0: f64,
    pub b0: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BoqRowView {
    #[serde(rename = "dv")]
    Division {
        key: String,
        label: String,
        sum: f64,
    },
    #[serde(rename = "sd")]
    Subdivision {
        key: String,
        code: String,
        name: String,
        label: String,
        sum: f64,
    },
    #[serde(rename = "line")]
    Line {
        no: usize,
        id: u32,
        k: String,
        u: String,
        s: String,
        qty: f64,
        note: String,
        custom: bool,
        man: bool,
        dv: String,
        g: String,
        rate: f64,
        sum: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoqSummary {
    pub rows: Vec<BoqRowView>,
    pub item_count: usize,
    pub subtotal: f64,
    pub ppn_pct: f64,
    pub ppn_amount: f64,
    pub grand_total: f64,
}

/// Normalizes "03 30 Cast..." into "03 30 00 Cast..."
pub fn norm_g(g: &str) -> String {
    let bytes = g.as_bytes();
    if bytes.len() >= 7
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b' '
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5] == b' '
        && !bytes[6].is_ascii_digit()
    {
        return format!("{} 00 {}", &g[..5], &g[6..]);
    }
    g.to_string()
}

pub fn split_subdiv(g: &str) -> (String, String) {
    let chars: Vec<(usize, char)> = g.char_indices().collect();
    let mut split_at = None;
    let mut saw_digit = false;
    for &(idx, ch) in &chars {
        if ch.is_ascii_digit() {
            saw_digit = true;
        } else if ch == ' ' {
            // keep scanning
        } else {
            if saw_digit {
                // find preceding space
                if idx > 0 && g.as_bytes()[idx - 1] == b' ' {
                    split_at = Some(idx);
                }
            }
            break;
        }
    }
    if let Some(pos) = split_at {
        (g[..pos].trim().to_string(), g[pos..].to_string())
    } else {
        (String::new(), g.to_string())
    }
}

fn group_sort_key(g: &str) -> String {
    let (code, _) = split_subdiv(g);
    if !code.is_empty() {
        code
    } else if !g.is_empty() {
        format!("zz{}", g)
    } else {
        String::new()
    }
}

pub fn effective_divs(db: &StaticDatabase, state: &ProjectState) -> HashMap<String, String> {
    let mut divs = db.payload.divs.clone();
    for (k, v) in &state.udivs {
        divs.entry(k.clone()).or_insert_with(|| v.clone());
    }
    divs
}

pub fn build_all_catalog(
    db: &StaticDatabase,
    ahs_items: &[AhsItem],
) -> Vec<CatalogEntry> {
    let mut out: Vec<CatalogEntry> = ahs_items
        .iter()
        .map(|it| CatalogEntry {
            k: it.k.clone(),
            u: it.u.clone(),
            s: it.s.clone(),
            g: it.g.clone(),
            dv: it.dv.clone(),
            m: false,
            usr: it.usr.is_some(),
            h: it.h,
            b: it.b,
            h0: it.h0,
            b0: it.b0,
        })
        .collect();

    for m in db.manual.man.iter().chain(db.manual.mep.iter()) {
        out.push(CatalogEntry {
            k: m.k.clone(),
            u: m.u.clone(),
            s: m.s.clone(),
            g: m.g.clone(),
            dv: m.dv.clone(),
            m: true,
            usr: false,
            h: 0.0,
            b: 0.0,
            h0: 0.0,
            b0: 0.0,
        });
    }

    out.sort_by(|a, b| {
        a.dv.cmp(&b.dv)
            .then_with(|| a.g.cmp(&b.g))
            .then_with(|| a.k.cmp(&b.k))
    });

    out
}

pub fn line_rate(line: &BoqLine, ahs_map: &HashMap<&str, &AhsItem>, basis: &str) -> f64 {
    if line.custom || line.man {
        line.price
    } else if let Some(it) = ahs_map.get(line.k.as_str()) {
        if basis == "b" {
            it.b
        } else {
            it.h
        }
    } else {
        0.0
    }
}

pub fn build_boq_summary(
    state: &ProjectState,
    ahs_items: &[AhsItem],
    catalog: &[CatalogEntry],
    divs: &HashMap<String, String>,
) -> BoqSummary {
    let ahs_map: HashMap<&str, &AhsItem> = ahs_items.iter().map(|it| (it.k.as_str(), it)).collect();
    let cat_map: HashMap<&str, &CatalogEntry> = catalog.iter().map(|c| (c.k.as_str(), c)).collect();

    struct SortItem<'a> {
        line: &'a BoqLine,
        dv: String,
        g: String,
        is_custom: u8,
        k: String,
        rate: f64,
        sum: f64,
    }

    let mut assigned: Vec<SortItem> = Vec::new();
    let mut unassigned_custom: Vec<(&BoqLine, f64, f64)> = Vec::new();

    for line in &state.lines {
        let r = line_rate(line, &ahs_map, &state.basis);
        let s = line.qty * r;
        if line.custom && line.dv.is_empty() {
            unassigned_custom.push((line, r, s));
        } else if line.custom {
            assigned.push(SortItem {
                line,
                dv: line.dv.clone(),
                g: line.g.trim().to_string(),
                is_custom: 1,
                k: String::new(),
                rate: r,
                sum: s,
            });
        } else {
            let (dv, g) = if let Some(c) = cat_map.get(line.k.as_str()) {
                (c.dv.clone(), c.g.clone())
            } else {
                (
                    line.k.chars().take(2).collect(),
                    line.k.chars().take(5).collect(),
                )
            };
            assigned.push(SortItem {
                line,
                dv,
                g,
                is_custom: 0,
                k: line.k.clone(),
                rate: r,
                sum: s,
            });
        }
    }

    assigned.sort_by(|a, b| {
        a.dv.cmp(&b.dv)
            .then_with(|| group_sort_key(&a.g).cmp(&group_sort_key(&b.g)))
            .then_with(|| a.is_custom.cmp(&b.is_custom))
            .then_with(|| a.k.cmp(&b.k))
    });

    // Precompute division and subdivision sums
    let mut dv_sums: HashMap<String, f64> = HashMap::new();
    let mut sd_sums: HashMap<(String, String), f64> = HashMap::new();
    for item in &assigned {
        *dv_sums.entry(item.dv.clone()).or_insert(0.0) += item.sum;
        if !item.g.is_empty() {
            *sd_sums
                .entry((item.dv.clone(), item.g.clone()))
                .or_insert(0.0) += item.sum;
        }
    }

    let mut rows = Vec::new();
    let mut cur_dv = String::new();
    let mut cur_sd = String::new();
    let mut no = 0usize;

    for item in assigned {
        if item.dv != cur_dv {
            cur_dv = item.dv.clone();
            cur_sd.clear();
            let dv_name = divs.get(&cur_dv).map(String::as_str).unwrap_or("");
            rows.push(BoqRowView::Division {
                key: cur_dv.clone(),
                label: format!("DIVISI {}  {}", cur_dv, dv_name),
                sum: *dv_sums.get(&cur_dv).unwrap_or(&0.0),
            });
        }
        if !item.g.is_empty() && item.g != cur_sd {
            cur_sd = item.g.clone();
            let (code, name) = split_subdiv(&cur_sd);
            rows.push(BoqRowView::Subdivision {
                key: cur_sd.clone(),
                code,
                name,
                label: cur_sd.clone(),
                sum: *sd_sums
                    .get(&(cur_dv.clone(), cur_sd.clone()))
                    .unwrap_or(&0.0),
            });
        }
        no += 1;
        rows.push(BoqRowView::Line {
            no,
            id: item.line.id,
            k: item.line.k.clone(),
            u: item.line.u.clone(),
            s: item.line.s.clone(),
            qty: item.line.qty,
            note: item.line.note.clone(),
            custom: item.line.custom,
            man: item.line.man,
            dv: item.line.dv.clone(),
            g: item.line.g.clone(),
            rate: item.rate,
            sum: item.sum,
        });
    }

    if !unassigned_custom.is_empty() {
        let extra_sum: f64 = unassigned_custom.iter().map(|(_, _, s)| *s).sum();
        rows.push(BoqRowView::Division {
            key: "XX".to_string(),
            label: "PEKERJAAN TAMBAHAN (ITEM CUSTOM)".to_string(),
            sum: extra_sum,
        });
        for (line, rate, sum) in unassigned_custom {
            no += 1;
            rows.push(BoqRowView::Line {
                no,
                id: line.id,
                k: line.k.clone(),
                u: line.u.clone(),
                s: line.s.clone(),
                qty: line.qty,
                note: line.note.clone(),
                custom: line.custom,
                man: line.man,
                dv: line.dv.clone(),
                g: line.g.clone(),
                rate,
                sum,
            });
        }
    }

    let subtotal: f64 = state
        .lines
        .iter()
        .map(|l| l.qty * line_rate(l, &ahs_map, &state.basis))
        .sum();
    let ppn_amount = subtotal * state.ppn / 100.0;

    BoqSummary {
        rows,
        item_count: state.lines.len(),
        subtotal,
        ppn_pct: state.ppn,
        ppn_amount,
        grand_total: subtotal + ppn_amount,
    }
}
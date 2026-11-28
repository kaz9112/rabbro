use std::fs;
use std::path::Path;

use crate::domain::boq::{BoqRowView, BoqSummary};
use crate::models::state::ProjectState;

pub fn format_id_date(date_str: &str) -> String {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return "-".to_string();
    }
    let parts: Vec<&str> = trimmed.split('-').collect();
    if parts.len() == 3 {
        let month = match parts[1] {
            "01" => "Januari",
            "02" => "Februari",
            "03" => "Maret",
            "04" => "April",
            "05" => "Mei",
            "06" => "Juni",
            "07" => "Juli",
            "08" => "Agustus",
            "09" => "September",
            "10" => "Oktober",
            "11" => "November",
            "12" => "Desember",
            _ => return trimmed.to_string(),
        };
        let day = parts[2].trim_start_matches('0');
        let day = if day.is_empty() { "0" } else { day };
        return format!("{} {} {}", day, month, parts[0]);
    }
    trimmed.to_string()
}

pub fn doc_rows(state: &ProjectState) -> [(&'static str, String); 5] {
    let or_dash = |s: &str| {
        if s.trim().is_empty() {
            "-".to_string()
        } else {
            s.to_string()
        }
    };
    [
        ("PROYEK", or_dash(&state.proj)),
        ("PERUSAHAAN", or_dash(&state.company)),
        ("DIBUAT OLEH", or_dash(&state.by)),
        ("TANGGAL DOKUMEN", format_id_date(&state.date)),
        ("REVISI NO", or_dash(&state.rev)),
    ]
}

fn table_rows(summary: &BoqSummary) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for row in &summary.rows {
        match row {
            BoqRowView::Division { label, sum, .. } => {
                out.push(vec![
                    String::new(),
                    String::new(),
                    label.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("{}", sum.round() as i64),
                ]);
            }
            BoqRowView::Subdivision { code, label, sum, .. } => {
                out.push(vec![
                    String::new(),
                    code.clone(),
                    label.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("{}", sum.round() as i64),
                ]);
            }
            BoqRowView::Line {
                no,
                k,
                u,
                s,
                qty,
                note,
                custom,
                rate,
                sum,
                ..
            } => {
                out.push(vec![
                    no.to_string(),
                    if *custom { "CUSTOM".to_string() } else { k.clone() },
                    u.clone(),
                    note.clone(),
                    s.clone(),
                    format!("{}", qty),
                    format!("{}", rate.round() as i64),
                    format!("{}", sum.round() as i64),
                ]);
            }
        }
    }

    out.push(vec![
        String::new(),
        String::new(),
        "Jumlah".to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        format!("{}", summary.subtotal.round() as i64),
    ]);
    out.push(vec![
        String::new(),
        String::new(),
        format!("PPN {}%", summary.ppn_pct),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        format!("{}", summary.ppn_amount.round() as i64),
    ]);
    out.push(vec![
        String::new(),
        String::new(),
        "Total RAB".to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        format!("{}", summary.grand_total.round() as i64),
    ]);

    out
}

pub fn build_boq_tsv(summary: &BoqSummary) -> String {
    let head = [
        "No",
        "Kode",
        "Uraian",
        "Keterangan",
        "Sat",
        "Volume",
        "Harga Satuan",
        "Jumlah",
    ]
    .join("\t");
    let mut lines = vec![head];
    for row in table_rows(summary) {
        lines.push(row.join("\t"));
    }
    lines.join("\n")
}

fn csv_cell(v: &str) -> String {
    format!("\"{}\"", v.replace('"', "\"\""))
}

pub fn build_boq_csv(state: &ProjectState, summary: &BoqSummary) -> String {
    let mut lines = Vec::new();
    lines.push(csv_cell("BILL OF QUANTITY"));
    for (lbl, val) in doc_rows(state) {
        lines.push(format!("{},{}", csv_cell(lbl), csv_cell(&val)));
    }
    lines.push(String::new());

    let head = [
        "No",
        "Kode",
        "Uraian",
        "Keterangan",
        "Sat",
        "Volume",
        "Harga Satuan",
        "Jumlah",
    ]
    .iter()
    .map(|h| csv_cell(h))
    .collect::<Vec<_>>()
    .join(",");
    lines.push(head);

    for row in table_rows(summary) {
        lines.push(
            row.iter()
                .map(|c| csv_cell(c))
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    format!("\u{FEFF}{}", lines.join("\r\n"))
}

pub fn write_boq_csv(
    path: &Path,
    state: &ProjectState,
    summary: &BoqSummary,
) -> Result<(), String> {
    let content = build_boq_csv(state, summary);
    fs::write(path, content).map_err(|e| format!("Gagal menulis CSV: {}", e))
}
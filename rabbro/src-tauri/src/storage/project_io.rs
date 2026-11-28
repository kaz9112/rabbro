use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ahs_builder::build_all_ahs_items;
use crate::domain::validation::sanitize_state;
use crate::models::catalog::StaticDatabase;
use crate::models::state::{ProjectFile, ProjectState};

const MAX_PROJECT_BYTES: u64 = 5_000_000;

pub fn default_export_basename(state: &ProjectState) -> String {
    let mut parts: Vec<String> = vec!["RAB".to_string()];
    if !state.proj.trim().is_empty() {
        parts.push(state.proj.trim().to_string());
    }
    if !state.company.trim().is_empty() {
        parts.push(state.company.trim().to_string());
    }
    if !state.rev.trim().is_empty() {
        parts.push(format!("Rev {}", state.rev.trim()));
    }
    let prefix = parts.join(" - ");
    let d: String = state.date.chars().filter(|c| *c != '-').collect();
    let auto = if d.is_empty() {
        prefix
    } else {
        format!("{} ({})", prefix, d)
    };

    let custom = state.fname.trim();
    let mut base = if custom.is_empty() {
        auto
    } else {
        custom
            .strip_suffix(".xlsx")
            .or_else(|| custom.strip_suffix(".csv"))
            .or_else(|| custom.strip_suffix(".json"))
            .unwrap_or(custom)
            .to_string()
    };

    if base.is_empty() {
        base = "RAB".to_string();
    }

    base.chars()
        .map(|c| {
            if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .take(150)
        .collect()
}

pub fn save_project_json(path: &Path, state: &ProjectState) -> Result<(), String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let file = ProjectFile {
        app: "rab-proto".to_string(),
        v: 1,
        saved: format!("unix:{}", secs),
        st: state.clone(),
    };

    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("Gagal serialisasi proyek: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Gagal menyimpan file: {}", e))?;
    Ok(())
}

pub fn open_project_json(
    path: &Path,
    db: &StaticDatabase,
) -> Result<(ProjectState, usize), String> {
    let meta = fs::metadata(path).map_err(|e| format!("Gagal membaca file: {}", e))?;
    if meta.len() > MAX_PROJECT_BYTES {
        return Err("File terlalu besar (maks 5 MB)".to_string());
    }

    let raw = fs::read_to_string(path).map_err(|e| format!("Gagal membaca file: {}", e))?;
    let parsed: ProjectFile =
        serde_json::from_str(&raw).map_err(|_| "Bukan file proyek RAB yang valid".to_string())?;

    if parsed.app != "rab-proto" {
        return Err("Bukan file proyek RAB".to_string());
    }

    let (clean, skipped) = sanitize_state(parsed.st, db);

    // Verify all AHS formulas in the loaded project evaluate without error
    build_all_ahs_items(db, &clean)
        .map_err(|e| format!("Perhitungan AHS pada file tidak valid: {}", e))?;

    Ok((clean, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_json_roundtrip_and_basename() {
        let db = StaticDatabase::load_embedded().unwrap();
        let mut state = ProjectState::default();
        state.proj = "Gudang Baru".into();
        state.company = "PT Contoh".into();
        state.rev = "1".into();
        state.date = "2026-11-27".into();

        assert_eq!(
            default_export_basename(&state),
            "RAB - Gudang Baru - PT Contoh - Rev 1 (20261127)"
        );

        let tmp = std::env::temp_dir().join("rabbro_test_roundtrip.json");
        save_project_json(&tmp, &state).unwrap();
        let (loaded, skipped) = open_project_json(&tmp, &db).unwrap();
        let _ = fs::remove_file(&tmp);

        assert_eq!(skipped, 0);
        assert_eq!(loaded.proj, "Gudang Baru");
        assert_eq!(loaded.company, "PT Contoh");
    }
}
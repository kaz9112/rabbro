use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::domain::validation::sanitize_state;
use crate::models::catalog::StaticDatabase;
use crate::models::state::{BoqLine, ProjectState};

pub fn autosave_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Gagal mengakses folder AppData: {}", e))?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Gagal membuat folder AppData: {}", e))?;
    Ok(dir.join("autosave.json"))
}

pub fn seed_example_lines(db: &StaticDatabase, state: &mut ProjectState) {
    if !state.lines.is_empty() {
        return;
    }
    let examples = [
        ("03 31 13.40 0100", 45.5, "Kolom lantai 1, as A-D"),
        ("03 21 11.20 0100", 5200.0, "Tulangan ulir D13-D19"),
        ("03 11 13.20 0100", 310.0, "Formwork kolom dan balok"),
    ];
    let mut id = 1u32;
    for (code, qty, note) in examples {
        if let Some(it) = db.payload.items.iter().find(|d| d.k == code) {
            state.lines.push(BoqLine {
                id,
                k: it.k.clone(),
                u: it.u.clone(),
                s: it.s.clone(),
                qty,
                note: note.to_string(),
                custom: false,
                man: false,
                price: 0.0,
                dv: String::new(),
                g: String::new(),
            });
            id += 1;
        }
    }
    state.lines.push(BoqLine {
        id,
        k: String::new(),
        u: "Pengujian kuat tekan beton (core drill), 3 titik".to_string(),
        s: "ls".to_string(),
        qty: 1.0,
        note: "Item di luar AHS".to_string(),
        custom: true,
        man: false,
        price: 4_500_000.0,
        dv: String::new(),
        g: String::new(),
    });
}

pub fn load_autosave_from_path(path: &Path, db: &StaticDatabase) -> ProjectState {
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<ProjectState>(&raw) {
            let (mut clean, _) = sanitize_state(parsed, db);
            seed_example_lines(db, &mut clean);
            return clean;
        }
    }
    let mut def = ProjectState::default();
    seed_example_lines(db, &mut def);
    def
}

pub fn save_autosave_to_path(path: &Path, state: &ProjectState) -> Result<(), String> {
    let json = serde_json::to_string(state)
        .map_err(|e| format!("Gagal serialisasi autosave: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("Gagal menulis autosave: {}", e))?;
    fs::rename(&tmp, path).map_err(|e| format!("Gagal mengganti file autosave: {}", e))?;
    Ok(())
}

pub fn load_autosave(app: &AppHandle, db: &StaticDatabase) -> ProjectState {
    match autosave_path(app) {
        Ok(p) => load_autosave_from_path(&p, db),
        Err(_) => {
            let mut def = ProjectState::default();
            seed_example_lines(db, &mut def);
            def
        }
    }
}

pub fn save_autosave(app: &AppHandle, state: &ProjectState) -> Result<(), String> {
    let p = autosave_path(app)?;
    save_autosave_to_path(&p, state)
}
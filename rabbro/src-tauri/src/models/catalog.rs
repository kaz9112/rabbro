use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CellValue {
    Number(f64),
    Text(String),
}

impl CellValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CellValue::Number(n) => Some(*n),
            CellValue::Text(_) => None,
        }
    }

    pub fn is_formula(&self) -> bool {
        matches!(self, CellValue::Text(s) if s.starts_with('='))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdRow {
    pub r: u32,
    pub k: String,
    pub n: String,
    pub u: String,
    #[serde(default)]
    pub c: String,
    #[serde(default)]
    pub f: bool,
    #[serde(default)]
    pub usr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhsRef {
    pub j: String,
    pub b: String,
    pub h: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhsItem {
    pub k: String,
    pub u: String,
    pub s: String,
    pub g: String,
    pub dv: String,
    pub d: String,
    #[serde(rename = "ref")]
    pub cell_ref: AhsRef,
    pub h0: f64,
    pub b0: f64,
    pub start: u32,
    pub end: u32,
    #[serde(default)]
    pub usr: Option<u32>,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub spec: bool,
    #[serde(default)]
    pub h: f64,
    #[serde(default)]
    pub b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub cells: HashMap<String, HashMap<String, CellValue>>,
    pub hd: Vec<HdRow>,
    pub items: Vec<AhsItem>,
    pub divs: HashMap<String, String>,
    pub names: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualItem {
    pub k: String,
    pub u: String,
    pub s: String,
    pub g: String,
    pub dv: String,
    pub m: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCatalogs {
    pub man: Vec<ManualItem>,
    pub mep: Vec<ManualItem>,
    pub extra_divs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StaticDatabase {
    pub payload: Payload,
    pub manual: ManualCatalogs,
}

impl StaticDatabase {
    pub fn load_embedded() -> Result<Self, serde_json::Error> {
        let mut payload: Payload =
            serde_json::from_str(include_str!("../../resources/payload.json"))?;
        let manual: ManualCatalogs =
            serde_json::from_str(include_str!("../../resources/manual_catalogs.json"))?;

        for (k, v) in &manual.extra_divs {
            payload.divs.entry(k.clone()).or_insert_with(|| v.clone());
        }

        for item in &mut payload.items {
            item.h = item.h0;
            item.b = item.b0;
        }

        Ok(Self { payload, manual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_json_deserializes_cleanly() {
        let db = StaticDatabase::load_embedded().expect("Failed to parse embedded JSON");
        assert!(!db.payload.items.is_empty(), "AHS items should not be empty");
        assert!(!db.payload.hd.is_empty(), "Harga Dasar rows should not be empty");
        assert!(db.payload.divs.contains_key("00"), "Extra divs should be merged");
        assert!(!db.manual.man.is_empty(), "MAN catalog should not be empty");
        assert!(!db.manual.mep.is_empty(), "MEP catalog should not be empty");
    }
}
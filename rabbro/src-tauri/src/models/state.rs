use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::catalog::{AhsRef, CellValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoqLine {
    pub id: u32,
    #[serde(default)]
    pub k: String,
    #[serde(default)]
    pub u: String,
    #[serde(default)]
    pub s: String,
    #[serde(default)]
    pub qty: f64,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub man: bool,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub dv: String,
    #[serde(default)]
    pub g: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewSpec {
    pub t: String,
    pub n: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub t: String,
    #[serde(default)]
    pub u: String,
    pub c: f64,
    #[serde(default)]
    pub p: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAhsSpec {
    pub crew: Vec<CrewSpec>,
    pub out: f64,
    pub mats: Vec<ComponentSpec>,
    pub eqs: Vec<ComponentSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAhs {
    pub id: u32,
    pub k: String,
    pub u: String,
    pub s: String,
    pub g: String,
    pub dv: String,
    pub start: u32,
    pub end: u32,
    #[serde(rename = "ref")]
    pub cell_ref: AhsRef,
    pub cells: HashMap<String, CellValue>,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub spec: Option<UserAhsSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserHdItem {
    pub id: u32,
    pub r: u32,
    pub k: String,
    pub n: String,
    pub u: String,
    #[serde(default)]
    pub c: String,
    pub v: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    #[serde(default)]
    pub fname: String,
    pub title: String,
    pub sub: String,
    pub proj: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub rev: String,
    pub basis: String,
    pub ppn: f64,
    #[serde(default)]
    pub lines: Vec<BoqLine>,
    #[serde(default)]
    pub hd: HashMap<String, f64>,
    #[serde(default)]
    pub ov: HashMap<String, CellValue>,
    #[serde(default)]
    pub uahs: Vec<UserAhs>,
    #[serde(default)]
    pub udivs: HashMap<String, String>,
    #[serde(default)]
    pub unext: u32,
    #[serde(default)]
    pub uhd: Vec<UserHdItem>,
    #[serde(default)]
    pub uhnext: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font")]
    pub font: String,
}

fn default_theme() -> String {
    "exist".to_string()
}

fn default_font() -> String {
    "exist".to_string()
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            fname: String::new(),
            title: "RabBro".to_string(),
            sub: "Based on CSI Masterformat 2020 dan RSMeans".to_string(),
            proj: "Mining Dormitory, Tanah Kunin".to_string(),
            company: String::new(),
            by: String::new(),
            date: String::new(),
            rev: "0".to_string(),
            basis: "h".to_string(),
            ppn: 11.0,
            lines: Vec::new(),
            hd: HashMap::new(),
            ov: HashMap::new(),
            uahs: Vec::new(),
            udivs: HashMap::new(),
            unext: 0,
            uhd: Vec::new(),
            uhnext: 0,
            theme: default_theme(),
            font: default_font(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub app: String,
    pub v: u32,
    #[serde(default)]
    pub saved: String,
    pub st: ProjectState,
}
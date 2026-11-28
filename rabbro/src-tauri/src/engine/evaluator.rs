use std::collections::{HashMap, HashSet};

use super::parser::{parse_formula, Expr};
use crate::models::catalog::CellValue;

#[derive(Debug, Clone)]
pub enum EvalValue {
    Number(f64),
    Text(String),
}

impl EvalValue {
    pub fn as_number(&self) -> Result<f64, String> {
        match self {
            EvalValue::Number(n) => Ok(*n),
            EvalValue::Text(_) => Err("Operasi aritmatika pada teks tidak didukung".into()),
        }
    }
}

pub struct FormulaEngine<'a> {
    cells: &'a HashMap<String, HashMap<String, CellValue>>,
    names: &'a HashMap<String, String>,
    cache: HashMap<String, EvalValue>,
    visiting: HashSet<String>,
    ast_cache: HashMap<String, Expr>,
}

impl<'a> FormulaEngine<'a> {
    pub fn new(
        cells: &'a HashMap<String, HashMap<String, CellValue>>,
        names: &'a HashMap<String, String>,
    ) -> Self {
        Self {
            cells,
            names,
            cache: HashMap::new(),
            visiting: HashSet::new(),
            ast_cache: HashMap::new(),
        }
    }

    pub fn get(&mut self, sheet: &str, addr: &str) -> Result<EvalValue, String> {
        let key = format!("{}|{}", sheet, addr);
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached.clone());
        }
        if !self.visiting.insert(key.clone()) {
            return Err(format!("Referensi berputar terdeteksi pada {}!{}", sheet, addr));
        }

        let result = self.eval_cell(sheet, addr);
        self.visiting.remove(&key);

        let val = result?;
        self.cache.insert(key, val.clone());
        Ok(val)
    }

    pub fn get_num(&mut self, sheet: &str, addr: &str) -> Result<f64, String> {
        self.get(sheet, addr)?.as_number()
    }

    fn eval_cell(&mut self, sheet: &str, addr: &str) -> Result<EvalValue, String> {
        let tab = self
            .cells
            .get(sheet)
            .ok_or_else(|| format!("Sheet tidak ditemukan: {}", sheet))?;

        match tab.get(addr) {
            None => Ok(EvalValue::Number(0.0)),
            Some(CellValue::Number(n)) => Ok(EvalValue::Number(*n)),
            Some(CellValue::Text(s)) => {
                if s.starts_with('=') {
                    let fk = format!("{}|{}", sheet, s);
                    if !self.ast_cache.contains_key(&fk) {
                        let ast = parse_formula(s, self.names)?;
                        self.ast_cache.insert(fk.clone(), ast);
                    }
                    let ast = self.ast_cache.get(&fk).unwrap().clone();
                    let num = self.eval_expr(&ast, sheet)?;
                    Ok(EvalValue::Number(num))
                } else {
                    Ok(EvalValue::Text(s.clone()))
                }
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr, cur_sheet: &str) -> Result<f64, String> {
        match expr {
            Expr::Number(n) => Ok(*n),
            Expr::Cell { sheet, addr } => {
                let sh = sheet.as_deref().unwrap_or(cur_sheet);
                self.get_num(sh, addr)
            }
            Expr::Range { .. } => Err("Rentang sel hanya dapat digunakan di dalam fungsi".into()),
            Expr::Unary { op, rhs } => {
                let v = self.eval_expr(rhs, cur_sheet)?;
                Ok(if *op == '-' { -v } else { v })
            }
            Expr::Binary { op, lhs, rhs } => {
                let l = self.eval_expr(lhs, cur_sheet)?;
                let r = self.eval_expr(rhs, cur_sheet)?;
                let res = match op {
                    '+' => l + r,
                    '-' => l - r,
                    '*' => l * r,
                    '/' => l / r,
                    '^' => l.powf(r),
                    _ => return Err(format!("Operator tidak dikenal: {}", op)),
                };
                if !res.is_finite() {
                    return Err("Hasil perhitungan tidak valid (NaN/Infinity)".into());
                }
                Ok(res)
            }
            Expr::Call { name, args } => self.eval_call(name, args, cur_sheet),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr], cur_sheet: &str) -> Result<f64, String> {
        match name {
            "PI" => Ok(std::f64::consts::PI),
            "ROUND" => {
                if args.len() != 2 {
                    return Err("ROUND membutuhkan 2 argumen".into());
                }
                let x = self.eval_expr(&args[0], cur_sheet)?;
                let n = self.eval_expr(&args[1], cur_sheet)?;
                let f = 10f64.powi(n as i32);
                let s = if x < 0.0 { -1.0 } else { 1.0 };
                Ok(s * (x.abs() * f + 1e-9).round() / f)
            }
            "SUM" => {
                let nums = self.flatten_args(args, cur_sheet)?;
                Ok(nums.into_iter().sum())
            }
            "MAX" => {
                let nums = self.flatten_args(args, cur_sheet)?;
                nums.into_iter()
                    .reduce(f64::max)
                    .ok_or_else(|| "MAX membutuhkan minimal 1 angka".into())
            }
            "MIN" => {
                let nums = self.flatten_args(args, cur_sheet)?;
                nums.into_iter()
                    .reduce(f64::min)
                    .ok_or_else(|| "MIN membutuhkan minimal 1 angka".into())
            }
            _ => Err(format!("Fungsi tidak didukung: {}", name)),
        }
    }

    fn flatten_args(&mut self, args: &[Expr], cur_sheet: &str) -> Result<Vec<f64>, String> {
        let mut out = Vec::new();
        for arg in args {
            match arg {
                Expr::Range { sheet, col1, row1, col2, row2 } => {
                    let sh = sheet.as_deref().unwrap_or(cur_sheet);
                    let c1 = col_to_num(col1);
                    let c2 = col_to_num(col2);
                    let (ca, cb) = (c1.min(c2), c1.max(c2));
                    let (ra, rb) = ((*row1).min(*row2), (*row1).max(*row2));
                    for c in ca..=cb {
                        let col_s = num_to_col(c);
                        for r in ra..=rb {
                            let addr = format!("{}{}", col_s, r);
                            if let EvalValue::Number(n) = self.get(sh, &addr)? {
                                out.push(n);
                            }
                        }
                    }
                }
                _ => out.push(self.eval_expr(arg, cur_sheet)?),
            }
        }
        Ok(out)
    }
}

fn col_to_num(col: &str) -> u32 {
    col.bytes().fold(0, |acc, b| acc * 26 + (b - b'A' + 1) as u32)
}

fn num_to_col(mut n: u32) -> String {
    let mut s = String::new();
    while n > 0 {
        let m = (n - 1) % 26;
        s.insert(0, (b'A' + m as u8) as char);
        n = (n - 1) / 26;
    }
    s
}
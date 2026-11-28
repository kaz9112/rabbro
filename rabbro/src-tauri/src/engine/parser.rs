use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Cell { sheet: Option<String>, addr: String },
    Range { sheet: Option<String>, col1: String, row1: u32, col2: String, row2: u32 },
    Unary { op: char, rhs: Box<Expr> },
    Binary { op: char, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    CellRef { sheet: Option<String>, addr: String },
    Op(char),
    LParen,
    RParen,
    Comma,
    Colon,
}

fn is_col_row(s: &str) -> Option<(String, u32)> {
    let clean: String = s.chars().filter(|&c| c != '$').collect();
    let idx = clean.find(|c: char| c.is_ascii_digit())?;
    let (col, row_str) = clean.split_at(idx);
    if col.is_empty() || col.len() > 2 || !col.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let row: u32 = row_str.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((col.to_ascii_uppercase(), row))
}

fn tokenize(input: &str, names: &HashMap<String, String>) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        // Quoted sheet prefix: 'Harga Dasar'!D7
        if ch == '\'' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '\'' {
                i += 1;
            }
            if i >= chars.len() {
                return Err("Tanda kutip sheet tidak tertutup".into());
            }
            let sheet_name: String = chars[start..i].iter().collect();
            i += 1; // skip closing '
            if i >= chars.len() || chars[i] != '!' {
                return Err(format!("Diharapkan '!' setelah nama sheet '{}'", sheet_name));
            }
            i += 1; // skip !

            let sheet_key = names
                .get(&sheet_name)
                .cloned()
                .ok_or_else(|| format!("Sheet tidak dikenal: {}", sheet_name))?;

            let ref_start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '$') {
                i += 1;
            }
            let raw_ref: String = chars[ref_start..i].iter().collect();
            let (col, row) = is_col_row(&raw_ref)
                .ok_or_else(|| format!("Referensi sel tidak valid: {}", raw_ref))?;
            tokens.push(Token::CellRef {
                sheet: Some(sheet_key),
                addr: format!("{}{}", col, row),
            });
            continue;
        }

        // Number literal
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // Scientific notation e.g. 1e-5
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let num_str: String = chars[start..i].iter().collect();
            let val: f64 = num_str
                .parse()
                .map_err(|_| format!("Angka tidak valid: {}", num_str))?;
            tokens.push(Token::Number(val));
            continue;
        }

        // Identifier or local CellRef ($D$27, D15, SUM, PI)
        if ch.is_ascii_alphabetic() || ch == '$' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '$') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let is_call = i < chars.len() && chars[i] == '(';
            if !is_call {
                if let Some((col, row)) = is_col_row(&ident) {
                    tokens.push(Token::CellRef {
                        sheet: None,
                        addr: format!("{}{}", col, row),
                    });
                    continue;
                }
            }
            tokens.push(Token::Ident(ident.to_ascii_uppercase()));
            continue;
        }

        match ch {
            '+' | '-' | '*' | '/' | '^' => tokens.push(Token::Op(ch)),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            ',' => tokens.push(Token::Comma),
            ':' => tokens.push(Token::Colon),
            _ => return Err(format!("Karakter tidak didukung dalam rumus: '{}'", ch)),
        }
        i += 1;
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul_div()?;
        while let Some(Token::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.next();
            let rhs = self.parse_mul_div()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        while let Some(Token::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.next();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if let Some(Token::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.next();
            let rhs = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                rhs: Box::new(rhs),
            });
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_primary()?;
        if let Some(Token::Op('^')) = self.peek().cloned() {
            self.next();
            let rhs = self.parse_unary()?;
            return Ok(Expr::Binary {
                op: '^',
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::LParen) => {
                let expr = self.parse_expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("Kurung tutup ')' hilang".into()),
                }
            }
            Some(Token::CellRef { sheet, addr }) => {
                if matches!(self.peek(), Some(Token::Colon)) {
                    self.next(); // skip :
                    match self.next() {
                        Some(Token::CellRef { sheet: sheet2, addr: addr2 }) => {
                            let (col1, row1) = is_col_row(&addr).unwrap();
                            let (col2, row2) = is_col_row(&addr2).unwrap();
                            Ok(Expr::Range {
                                sheet: sheet.or(sheet2),
                                col1,
                                row1,
                                col2,
                                row2,
                            })
                        }
                        _ => Err("Diharapkan sel akhir setelah ':'".into()),
                    }
                } else {
                    Ok(Expr::Cell { sheet, addr })
                }
            }
            Some(Token::Ident(name)) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next(); // skip (
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expr()?);
                            match self.peek() {
                                Some(Token::Comma) => {
                                    self.next();
                                }
                                Some(Token::RParen) => break,
                                _ => return Err("Diharapkan ',' atau ')' dalam fungsi".into()),
                            }
                        }
                    }
                    self.next(); // skip )
                    Ok(Expr::Call { name, args })
                } else {
                    Err(format!("Identifier tidak dikenal: {}", name))
                }
            }
            Some(t) => Err(format!("Token tidak terduga: {:?}", t)),
            None => Err("Rumus tidak lengkap".into()),
        }
    }
}

pub fn parse_formula(formula: &str, names: &HashMap<String, String>) -> Result<Expr, String> {
    let body = formula.strip_prefix('=').unwrap_or(formula);
    let tokens = tokenize(body, names)?;
    if tokens.is_empty() {
        return Err("Rumus kosong".into());
    }
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err("Ada bagian rumus yang tidak dapat dibaca".into());
    }
    Ok(expr)
}

#[derive(Debug, Clone)]
pub enum ParsedInput {
    Delete,
    Number(f64),
    Formula(String),
}

pub fn parse_user_input(txt: &str, names: &HashMap<String, String>) -> Result<ParsedInput, String> {
    let t = txt.trim();
    if t.is_empty() {
        return Ok(ParsedInput::Delete);
    }
    if t.starts_with('=') {
        let normalized = t.replace("HD!", "'Harga Dasar'!");
        let expr = parse_formula(&normalized, names)?;
        validate_functions(&expr)?;
        return Ok(ParsedInput::Formula(normalized));
    }

    let mut num_str: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(comma_idx) = num_str.find(',') {
        let (left, right) = num_str.split_at(comma_idx);
        let right_digits = &right[1..];
        let left_ok = left.strip_prefix('-').unwrap_or(left).chars().all(|c| c.is_ascii_digit());
        if !left.is_empty() && left_ok && !right_digits.is_empty() && right_digits.chars().all(|c| c.is_ascii_digit()) {
            num_str = format!("{}.{}", left, right_digits);
        }
    }

    let val: f64 = num_str
        .parse()
        .map_err(|_| "Isi angka atau rumus yang diawali =".to_string())?;
    if !val.is_finite() {
        return Err("Angka tidak valid".into());
    }
    Ok(ParsedInput::Number(val))
}

fn validate_functions(expr: &Expr) -> Result<(), String> {
    match expr {
        Expr::Call { name, args } => {
            if !matches!(name.as_str(), "SUM" | "ROUND" | "PI" | "MAX" | "MIN") {
                return Err("Fungsi yang didukung: SUM, ROUND, PI, MAX, MIN".into());
            }
            for a in args {
                validate_functions(a)?;
            }
            Ok(())
        }
        Expr::Unary { rhs, .. } => validate_functions(rhs),
        Expr::Binary { lhs, rhs, .. } => {
            validate_functions(lhs)?;
            validate_functions(rhs)
        }
        _ => Ok(()),
    }
}
// Word expansion

use std::collections::HashMap;

const MAX_ARITH_DEPTH: usize = 64;
const MAX_EXPAND_DEPTH: usize = 32;

// Simple arithmetic evaluator (for $(()) and declare -i)
pub fn eval_arith_simple(expr: &str) -> Result<i64, String> {
    let expr = expr.trim();
    // Use empty var context
    ARITH_VARS.with(|v| {
        *v.borrow_mut() = Some(std::collections::HashMap::new());
    });
    ARITH_VARSTORE_PTR.with(|ptr| {
        *ptr.borrow_mut() = std::ptr::null();
    });
    let result = eval_arith_expr(expr);
    ARITH_VARS.with(|v| {
        *v.borrow_mut() = None;
    });
    ARITH_VARSTORE_PTR.with(|ptr| {
        *ptr.borrow_mut() = std::ptr::null();
    });
    result
}

fn eval_arith_expr(expr: &str) -> Result<i64, String> {
    ARITH_DEPTH.with(|d| {
        *d.borrow_mut() = 0;
    });
    let tokens = arith_tokenize(expr)?;
    let mut pos = 0;
    let result = arith_parse_expr(&tokens, &mut pos)?;
    ARITH_DEPTH.with(|d| {
        *d.borrow_mut() = 0;
    });
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
enum ATok {
    Num(i64),
    Var(String),
    Plus, Minus, Star, Slash, Percent, StarStar,
    Amp, Pipe, Caret, Tilde, Bang,
    AmpAmp, PipePipe,
    EqEq, BangEq,
    Lt, Gt, LtEq, GtEq,
    LShift, RShift,
    LParen, RParen,
    Question, Colon,
    // Assignment operators
    Eq,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    AmpEq, PipeEq, CaretEq, LShiftEq, RShiftEq,
    // Increment/decrement
    PlusPlus, MinusMinus,
}

fn arith_tokenize(s: &str) -> Result<Vec<ATok>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' => { i += 1; }
            '0'..='9' => {
                let mut n = String::new();
                // Hex
                if chars[i] == '0' && i + 1 < chars.len() && (chars[i+1] == 'x' || chars[i+1] == 'X') {
                    i += 2;
                    while i < chars.len() && chars[i].is_ascii_hexdigit() {
                        n.push(chars[i]);
                        i += 1;
                    }
                    let v = i64::from_str_radix(&n, 16).unwrap_or(0);
                    tokens.push(ATok::Num(v));
                } else if chars[i] == '0' && i + 1 < chars.len() && chars[i+1].is_ascii_digit() {
                    // Octal
                    i += 1;
                    while i < chars.len() && chars[i] >= '0' && chars[i] <= '7' {
                        n.push(chars[i]);
                        i += 1;
                    }
                    let v = i64::from_str_radix(&n, 8).unwrap_or(0);
                    tokens.push(ATok::Num(v));
                } else {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        n.push(chars[i]);
                        i += 1;
                    }
                    let v: i64 = n.parse().unwrap_or(0);
                    tokens.push(ATok::Num(v));
                }
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut name = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    name.push(chars[i]);
                    i += 1;
                }
                tokens.push(ATok::Var(name));
            }
            '+' => {
                if i + 1 < chars.len() && chars[i+1] == '+' {
                    tokens.push(ATok::PlusPlus); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::PlusEq); i += 2;
                } else {
                    tokens.push(ATok::Plus); i += 1;
                }
            }
            '-' => {
                if i + 1 < chars.len() && chars[i+1] == '-' {
                    tokens.push(ATok::MinusMinus); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::MinusEq); i += 2;
                } else {
                    tokens.push(ATok::Minus); i += 1;
                }
            }
            '*' => {
                if i + 1 < chars.len() && chars[i+1] == '*' {
                    tokens.push(ATok::StarStar); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::StarEq); i += 2;
                } else {
                    tokens.push(ATok::Star); i += 1;
                }
            }
            '/' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::SlashEq); i += 2;
                } else {
                    tokens.push(ATok::Slash); i += 1;
                }
            }
            '%' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::PercentEq); i += 2;
                } else {
                    tokens.push(ATok::Percent); i += 1;
                }
            }
            '&' => {
                if i + 1 < chars.len() && chars[i+1] == '&' {
                    tokens.push(ATok::AmpAmp); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::AmpEq); i += 2;
                } else {
                    tokens.push(ATok::Amp); i += 1;
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i+1] == '|' {
                    tokens.push(ATok::PipePipe); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::PipeEq); i += 2;
                } else {
                    tokens.push(ATok::Pipe); i += 1;
                }
            }
            '^' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::CaretEq); i += 2;
                } else {
                    tokens.push(ATok::Caret); i += 1;
                }
            }
            '~' => { tokens.push(ATok::Tilde); i += 1; }
            '!' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::BangEq); i += 2;
                } else {
                    tokens.push(ATok::Bang); i += 1;
                }
            }
            '=' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::EqEq); i += 2;
                } else {
                    tokens.push(ATok::Eq); i += 1;
                }
            }
            '<' => {
                if i + 2 < chars.len() && chars[i+1] == '<' && chars[i+2] == '=' {
                    tokens.push(ATok::LShiftEq); i += 3;
                } else if i + 1 < chars.len() && chars[i+1] == '<' {
                    tokens.push(ATok::LShift); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::LtEq); i += 2;
                } else {
                    tokens.push(ATok::Lt); i += 1;
                }
            }
            '>' => {
                if i + 2 < chars.len() && chars[i+1] == '>' && chars[i+2] == '=' {
                    tokens.push(ATok::RShiftEq); i += 3;
                } else if i + 1 < chars.len() && chars[i+1] == '>' {
                    tokens.push(ATok::RShift); i += 2;
                } else if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(ATok::GtEq); i += 2;
                } else {
                    tokens.push(ATok::Gt); i += 1;
                }
            }
            '(' => { tokens.push(ATok::LParen); i += 1; }
            ')' => { tokens.push(ATok::RParen); i += 1; }
            '?' => { tokens.push(ATok::Question); i += 1; }
            ':' => { tokens.push(ATok::Colon); i += 1; }
            _ => { i += 1; }
        }
    }
    Ok(tokens)
}

fn arith_parse_expr(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let depth_exceeded = ARITH_DEPTH.with(|d| {
        let mut depth = d.borrow_mut();
        if *depth >= MAX_ARITH_DEPTH {
            return true;
        }
        *depth += 1;
        false
    });
    if depth_exceeded {
        return Err("Arithmetic nesting too deep".to_string());
    }
    let result = arith_parse_assign(tokens, pos);
    ARITH_DEPTH.with(|d| {
        *d.borrow_mut() -= 1;
    });
    result
}

fn arith_var_read(name: &str) -> i64 {
    // First check if it was assigned in this arithmetic evaluation (stored in ARITH_VARS)
    if let Some(val_str) = ARITH_VARS.with(|v| {
        v.borrow().as_ref().and_then(|map| map.get(name).cloned())
    }) {
        return val_str.trim().parse().unwrap_or(0);
    }

    // Otherwise, look up directly from the VarStore (avoids cloning all variables)
    let val = ARITH_VARSTORE_PTR.with(|ptr| {
        let varstore_ptr = *ptr.borrow();
        if !varstore_ptr.is_null() {
            unsafe {
                (*varstore_ptr).get_str(name).unwrap_or_default()
            }
        } else {
            String::new()
        }
    });
    val.trim().parse().unwrap_or(0)
}

fn arith_var_write(name: &str, value: i64) {
    let val_str = value.to_string();
    ARITH_VARS.with(|v| {
        if let Some(map) = &mut *v.borrow_mut() {
            map.insert(name.to_string(), val_str.clone());
        }
    });
    push_param_assign(name.to_string(), val_str);
}

fn arith_apply_assign_op(name: &str, op: &ATok, rhs: i64) -> Result<i64, String> {
    let cur = arith_var_read(name);
    let new_val = match op {
        ATok::Eq        => rhs,
        ATok::PlusEq    => cur.wrapping_add(rhs),
        ATok::MinusEq   => cur.wrapping_sub(rhs),
        ATok::StarEq    => cur.wrapping_mul(rhs),
        ATok::SlashEq   => { if rhs == 0 { return Err("division by zero".to_string()); } cur / rhs }
        ATok::PercentEq => { if rhs == 0 { return Err("division by zero".to_string()); } cur % rhs }
        ATok::AmpEq     => cur & rhs,
        ATok::PipeEq    => cur | rhs,
        ATok::CaretEq   => cur ^ rhs,
        ATok::LShiftEq  => { let s = rhs.clamp(0, 63) as u32; cur << s }
        ATok::RShiftEq  => { let s = rhs.clamp(0, 63) as u32; cur >> s }
        _ => rhs,
    };
    arith_var_write(name, new_val);
    Ok(new_val)
}

// Assignment operators: lowest precedence, right-associative, LHS must be a variable
fn arith_parse_assign(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    if *pos < tokens.len() {
        if let ATok::Var(name) = &tokens[*pos] {
            if *pos + 1 < tokens.len() {
                let is_assign_op = matches!(
                    tokens[*pos + 1],
                    ATok::Eq | ATok::PlusEq | ATok::MinusEq | ATok::StarEq |
                    ATok::SlashEq | ATok::PercentEq | ATok::AmpEq | ATok::PipeEq |
                    ATok::CaretEq | ATok::LShiftEq | ATok::RShiftEq
                );
                if is_assign_op {
                    let name = name.clone();
                    let op = tokens[*pos + 1].clone();
                    *pos += 2;
                    let rhs = arith_parse_assign(tokens, pos)?;
                    return arith_apply_assign_op(&name, &op, rhs);
                }
            }
        }
    }
    arith_parse_ternary(tokens, pos)
}

fn arith_parse_ternary(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let cond = arith_parse_or(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos] == ATok::Question {
        *pos += 1;
        let t = arith_parse_expr(tokens, pos)?;
        if *pos < tokens.len() && tokens[*pos] == ATok::Colon {
            *pos += 1;
        }
        let f = arith_parse_expr(tokens, pos)?;
        return Ok(if cond != 0 { t } else { f });
    }
    Ok(cond)
}

fn arith_parse_or(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_and(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == ATok::PipePipe {
        *pos += 1;
        let right = arith_parse_and(tokens, pos)?;
        left = if left != 0 || right != 0 { 1 } else { 0 };
    }
    Ok(left)
}

fn arith_parse_and(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_bitor(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == ATok::AmpAmp {
        *pos += 1;
        let right = arith_parse_bitor(tokens, pos)?;
        left = if left != 0 && right != 0 { 1 } else { 0 };
    }
    Ok(left)
}

fn arith_parse_bitor(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_bitxor(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == ATok::Pipe {
        *pos += 1;
        let right = arith_parse_bitxor(tokens, pos)?;
        left |= right;
    }
    Ok(left)
}

fn arith_parse_bitxor(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_bitand(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == ATok::Caret {
        *pos += 1;
        let right = arith_parse_bitand(tokens, pos)?;
        left ^= right;
    }
    Ok(left)
}

fn arith_parse_bitand(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_eq(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == ATok::Amp {
        *pos += 1;
        let right = arith_parse_eq(tokens, pos)?;
        left &= right;
    }
    Ok(left)
}

fn arith_parse_eq(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_cmp(tokens, pos)?;
    loop {
        if *pos < tokens.len() && tokens[*pos] == ATok::EqEq {
            *pos += 1;
            let right = arith_parse_cmp(tokens, pos)?;
            left = if left == right { 1 } else { 0 };
        } else if *pos < tokens.len() && tokens[*pos] == ATok::BangEq {
            *pos += 1;
            let right = arith_parse_cmp(tokens, pos)?;
            left = if left != right { 1 } else { 0 };
        } else {
            break;
        }
    }
    Ok(left)
}

fn arith_parse_cmp(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_shift(tokens, pos)?;
    loop {
        if *pos >= tokens.len() { break; }
        match tokens[*pos] {
            ATok::Lt   => { *pos += 1; let r = arith_parse_shift(tokens, pos)?; left = if left < r { 1 } else { 0 }; }
            ATok::Gt   => { *pos += 1; let r = arith_parse_shift(tokens, pos)?; left = if left > r { 1 } else { 0 }; }
            ATok::LtEq => { *pos += 1; let r = arith_parse_shift(tokens, pos)?; left = if left <= r { 1 } else { 0 }; }
            ATok::GtEq => { *pos += 1; let r = arith_parse_shift(tokens, pos)?; left = if left >= r { 1 } else { 0 }; }
            _ => break,
        }
    }
    Ok(left)
}

fn arith_parse_shift(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_add(tokens, pos)?;
    loop {
        if *pos >= tokens.len() { break; }
        match tokens[*pos] {
            ATok::LShift => { *pos += 1; let r = arith_parse_add(tokens, pos)?; let shift = (r as i64).clamp(0, 63) as u32; left <<= shift; }
            ATok::RShift => { *pos += 1; let r = arith_parse_add(tokens, pos)?; let shift = (r as i64).clamp(0, 63) as u32; left >>= shift; }
            _ => break,
        }
    }
    Ok(left)
}

fn arith_parse_add(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_mul(tokens, pos)?;
    loop {
        if *pos >= tokens.len() { break; }
        match tokens[*pos] {
            ATok::Plus  => { *pos += 1; let r = arith_parse_mul(tokens, pos)?; left = left.wrapping_add(r); }
            ATok::Minus => { *pos += 1; let r = arith_parse_mul(tokens, pos)?; left = left.wrapping_sub(r); }
            _ => break,
        }
    }
    Ok(left)
}

fn arith_parse_mul(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let mut left = arith_parse_pow(tokens, pos)?;
    loop {
        if *pos >= tokens.len() { break; }
        match tokens[*pos] {
            ATok::Star    => { *pos += 1; let r = arith_parse_pow(tokens, pos)?; left = left.wrapping_mul(r); }
            ATok::Slash   => { *pos += 1; let r = arith_parse_pow(tokens, pos)?; if r == 0 { return Err("division by zero".to_string()); } left /= r; }
            ATok::Percent => { *pos += 1; let r = arith_parse_pow(tokens, pos)?; if r == 0 { return Err("division by zero".to_string()); } left %= r; }
            _ => break,
        }
    }
    Ok(left)
}

fn arith_parse_pow(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    let base = arith_parse_unary(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos] == ATok::StarStar {
        *pos += 1;
        let exp = arith_parse_unary(tokens, pos)?;
        Ok(base.wrapping_pow(exp as u32))
    } else {
        Ok(base)
    }
}

fn arith_parse_unary(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    if *pos >= tokens.len() {
        return Ok(0);
    }
    match &tokens[*pos] {
        ATok::Minus => {
            *pos += 1;
            let v = arith_parse_unary(tokens, pos)?;
            Ok(-v)
        }
        ATok::Plus => {
            *pos += 1;
            arith_parse_unary(tokens, pos)
        }
        ATok::Tilde => {
            *pos += 1;
            let v = arith_parse_unary(tokens, pos)?;
            Ok(!v)
        }
        ATok::Bang => {
            *pos += 1;
            let v = arith_parse_unary(tokens, pos)?;
            Ok(if v == 0 { 1 } else { 0 })
        }
        ATok::PlusPlus => {
            *pos += 1;
            if *pos < tokens.len() {
                if let ATok::Var(name) = &tokens[*pos] {
                    let name = name.clone();
                    *pos += 1;
                    let new_val = arith_var_read(&name).wrapping_add(1);
                    arith_var_write(&name, new_val);
                    return Ok(new_val);
                }
            }
            Ok(0)
        }
        ATok::MinusMinus => {
            *pos += 1;
            if *pos < tokens.len() {
                if let ATok::Var(name) = &tokens[*pos] {
                    let name = name.clone();
                    *pos += 1;
                    let new_val = arith_var_read(&name).wrapping_sub(1);
                    arith_var_write(&name, new_val);
                    return Ok(new_val);
                }
            }
            Ok(0)
        }
        _ => arith_parse_primary(tokens, pos),
    }
}

// Global vars reference for arith (only used when called from eval_arith_simple)
thread_local! {
    // Only for variables modified during arithmetic evaluation (assignments inside $(()), not for reading)
    static ARITH_VARS: std::cell::RefCell<Option<std::collections::HashMap<String, String>>> =
        std::cell::RefCell::new(None);
    // Pointer to VarStore for efficient variable lookup during arithmetic (avoids cloning all vars)
    static ARITH_VARSTORE_PTR: std::cell::RefCell<*const crate::shell::vars::VarStore> =
        std::cell::RefCell::new(std::ptr::null());
    static ARITH_DEPTH: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
    static EXPAND_DEPTH: std::cell::RefCell<usize> = std::cell::RefCell::new(0);
    // Set to true when ${var:?err} triggers
    pub static PARAM_ERROR: std::cell::RefCell<bool> = std::cell::RefCell::new(false);
    pub static PARAM_ASSIGN: std::cell::RefCell<Vec<(String, String)>> = std::cell::RefCell::new(Vec::new());
    // Last command substitution $() exit status
    pub static LAST_CMDSUB_STATUS: std::cell::RefCell<Option<i32>> = std::cell::RefCell::new(None);
}

pub fn take_cmdsub_status() -> Option<i32> {
    LAST_CMDSUB_STATUS.with(|s| s.borrow_mut().take())
}

pub fn take_param_error() -> bool {
    PARAM_ERROR.with(|e| {
        let v = *e.borrow();
        *e.borrow_mut() = false;
        v
    })
}

pub fn push_param_assign(name: String, value: String) {
    PARAM_ASSIGN.with(|a| {
        a.borrow_mut().push((name, value));
    });
}

pub fn take_param_assigns() -> Vec<(String, String)> {
    PARAM_ASSIGN.with(|a| {
        let v = a.borrow().clone();
        a.borrow_mut().clear();
        v
    })
}

fn arith_parse_primary(tokens: &[ATok], pos: &mut usize) -> Result<i64, String> {
    if *pos >= tokens.len() {
        return Ok(0);
    }
    match &tokens[*pos] {
        ATok::Num(n) => {
            let v = *n;
            *pos += 1;
            Ok(v)
        }
        ATok::Var(name) => {
            let name = name.clone();
            *pos += 1;
            let val = arith_var_read(&name);
            // Post-increment/decrement: return old value, write new value
            if *pos < tokens.len() && tokens[*pos] == ATok::PlusPlus {
                *pos += 1;
                arith_var_write(&name, val.wrapping_add(1));
                return Ok(val);
            }
            if *pos < tokens.len() && tokens[*pos] == ATok::MinusMinus {
                *pos += 1;
                arith_var_write(&name, val.wrapping_sub(1));
                return Ok(val);
            }
            Ok(val)
        }
        ATok::LParen => {
            *pos += 1;
            let v = arith_parse_expr(tokens, pos)?;
            if *pos < tokens.len() && tokens[*pos] == ATok::RParen {
                *pos += 1;
            }
            Ok(v)
        }
        _ => Ok(0),
    }
}

// Check if a word token (with quote markers like "$DIR"/*.sh) has any unquoted glob chars
fn has_unquoted_glob_chars(word: &str) -> bool {
    let chars: Vec<char> = word.chars().collect();
    let mut i = 0;
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;

    while i < chars.len() {
        match chars[i] {
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
                i += 1;
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
                i += 1;
            }
            '\\' if !in_single_quotes => {
                i += 2; // Skip escape sequence
            }
            '*' | '?' if !in_single_quotes && !in_double_quotes => {
                return true;
            }
            '[' if !in_single_quotes && !in_double_quotes => {
                // Check if this looks like a bracket expression (not escaped, has content)
                if i + 1 < chars.len() && chars[i + 1] != ' ' {
                    return true;
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    false
}

// Expand a word token, returning list of strings (after word splitting + glob)
pub fn expand_word(word: &str, quoted: bool, vars: &crate::shell::vars::VarStore, script_file: &str) -> Vec<String> {
    // Special case: "$@" or "${@}" inside double quotes should expand to multiple words
    if quoted && (word == "\"$@\"" || word == "\"${@}\"") {
        let count_str = vars.get_str("#").unwrap_or_else(|| "0".to_string());
        let count: usize = count_str.parse().unwrap_or(0);
        let mut result = Vec::new();
        for idx in 1..=count {
            if let Some(val) = vars.get_str(&idx.to_string()) {
                result.push(val);
            }
        }
        return result;
    }

    // Special case: "${arr[@]}" inside double quotes should expand to multiple words
    // Also handles "${arr[@]/pattern/replacement}" syntax
    // But NOT "${#arr[@]}" which is array length (different expansion)
    if quoted && word.starts_with("\"${") && word.ends_with("}\"") {
        let inner = &word[3..word.len()-2]; // Strip "$ { and }"

        // Check if it's a pattern substitution ${arr[@]/pat/repl}
        if let Some(slash_pos) = inner.find('/') {
            if inner[..slash_pos].ends_with("[@]") {
                let arr_name_with_bracket = &inner[..slash_pos];
                let arr_name = &arr_name_with_bracket[..arr_name_with_bracket.len()-3];
                let rest = &inner[slash_pos+1..];
                let (global, rest) = if rest.starts_with('/') {
                    (true, &rest[1..])
                } else {
                    (false, rest)
                };
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                let pat = parts[0];
                let repl = if parts.len() > 1 { parts[1] } else { "" };

                if let Some(arr) = vars.get_array(arr_name) {
                    let mut keys: Vec<usize> = arr.keys().copied().collect();
                    keys.sort();
                    // Expand variables in the replacement string
                    let expanded_repl = expand_string(repl, vars, script_file);
                    let mut result = Vec::new();
                    for k in keys {
                        result.push(replace_pattern(&arr[&k], pat, &expanded_repl, global));
                    }
                    return result;
                }
                // Array doesn't exist, return empty
                return vec![];
            }
        }

        if inner.ends_with("[@]") && !inner.starts_with('#') {
            let arr_name = &inner[..inner.len()-3];
            if let Some(arr) = vars.get_array(arr_name) {
                let mut keys: Vec<usize> = arr.keys().copied().collect();
                keys.sort();
                let mut result = Vec::new();
                for k in keys {
                    result.push(arr[&k].clone());
                }
                return result;
            }
            // Array doesn't exist, fall through to normal expansion
        }
    }

    let expanded = expand_word_no_split(word, quoted, vars, script_file);

    // Only skip word-splitting and globbing if the ENTIRE word was quoted.
    // If the word has any unquoted parts (like "$DIR"/*.sh), we should still glob.
    let should_skip_splitting = quoted && !has_unquoted_glob_chars(word);

    if should_skip_splitting {
        return vec![expanded];
    }

    // Word split
    let ifs = vars.get_str("IFS").unwrap_or_else(|| " \t\n".to_string());
    let parts = word_split(&expanded, &ifs);

    // Glob expansion
    let mut result = Vec::new();
    for part in parts {
        let globbed = glob_expand(&part);
        result.extend(globbed);
    }

    if result.is_empty() && !expanded.is_empty() {
        // Keep empty string if the expansion produced nothing useful
    }

    result
}

pub fn expand_word_no_glob(word: &str, quoted: bool, vars: &crate::shell::vars::VarStore, script_file: &str) -> Vec<String> {
    // The [[ ... ]] construct is lexed as a single token (bracket mode captures everything).
    // We must split it by unquoted whitespace first, then expand each sub-token individually.
    // This preserves empty variable expansions (e.g. $x when x="").
    if word.starts_with("[[") {
        return expand_bracket_test_token(word, vars, script_file);
    }

    // For test/[ commands, each token is already a separate word.
    // Expand without word splitting or globbing — empty variables produce "".
    if !quoted {
        return vec![expand_string(word, vars, script_file)];
    }

    // Quoted token: single-element expansion (quotes already stripped by expand_string)
    vec![expand_string(word, vars, script_file)]
}

fn expand_bracket_test_token(word: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> Vec<String> {
    // Split the [[ ... ]] single-token by unquoted whitespace, then expand each part.
    // This correctly handles empty variable expansions within [[ ]] tests.
    let chars: Vec<char> = word.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut current = String::new();
    let mut in_token = false;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' if !in_double_quote && !in_single_quote => {
                if in_token {
                    result.push(expand_string(&current, vars, script_file));
                    current.clear();
                    in_token = false;
                }
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(c);
                in_token = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(c);
                in_token = true;
            }
            '\\' if !in_single_quote => {
                current.push(c);
                in_token = true;
                i += 1;
                if i < chars.len() {
                    current.push(chars[i]);
                }
            }
            _ => {
                current.push(c);
                in_token = true;
            }
        }
        i += 1;
    }
    if in_token {
        result.push(expand_string(&current, vars, script_file));
    }
    result
}

pub fn expand_word_no_split(word: &str, _quoted: bool, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    expand_string(word, vars, script_file)
}

pub fn expand_string(s: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    let depth_exceeded = EXPAND_DEPTH.with(|d| {
        let mut depth = d.borrow_mut();
        if *depth >= MAX_EXPAND_DEPTH {
            return true;
        }
        *depth += 1;
        false
    });
    if depth_exceeded {
        return String::new();
    }

    let result = expand_string_inner(s, vars, script_file);

    EXPAND_DEPTH.with(|d| {
        *d.borrow_mut() -= 1;
    });
    result
}

pub fn expand_heredoc_body(s: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    let depth_exceeded = EXPAND_DEPTH.with(|d| {
        let mut depth = d.borrow_mut();
        if *depth >= MAX_EXPAND_DEPTH {
            return true;
        }
        *depth += 1;
        false
    });
    if depth_exceeded {
        return String::new();
    }

    let result = expand_heredoc_body_inner(s, vars, script_file);

    EXPAND_DEPTH.with(|d| {
        *d.borrow_mut() -= 1;
    });
    result
}

fn expand_heredoc_body_inner(s: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '"' => {
                // In heredoc body, quotes are literal characters, not delimiters.
                // Just add them as-is.
                result.push('"');
                i += 1;
            }
            '\\' => {
                i += 1;
                if i < chars.len() {
                    match chars[i] {
                        '$' | '`' | '\\' | '"' | '\n' => {
                            // These are escape sequences in a heredoc body with active expansion
                            if chars[i] != '\n' {
                                result.push(chars[i]);
                            }
                            i += 1;
                        }
                        _ => {
                            // Other backslashes are literal
                            result.push('\\');
                        }
                    }
                }
            }
            '$' => {
                let (expanded, consumed) = expand_dollar(&chars, i, vars, script_file);
                result.push_str(&expanded);
                i += consumed;
            }
            '`' => {
                let (expanded, consumed) = expand_backtick(&chars, i, vars, script_file);
                result.push_str(&expanded);
                i += consumed;
            }
            '~' => {
                // Tilde expansion at start or after :
                let tilde_result = expand_tilde(&chars, i, vars);
                result.push_str(&tilde_result.0);
                i += tilde_result.1;
            }
            '\'' => {
                // In heredoc body, single quotes are literal characters, not delimiters.
                result.push('\'');
                i += 1;
            }
            c => {
                result.push(c);
                i += 1;
            }
        }
    }
    result
}

fn expand_string_inner(s: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '"' => {
                // Double-quoted section - expand $ and ` inside
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    match chars[i] {
                        '$' => {
                            let (expanded, consumed) = expand_dollar(&chars, i, vars, script_file);
                            result.push_str(&expanded);
                            i += consumed;
                        }
                        '`' => {
                            let (expanded, consumed) = expand_backtick(&chars, i, vars, script_file);
                            result.push_str(&expanded);
                            i += consumed;
                        }
                        '\\' => {
                            i += 1;
                            if i < chars.len() {
                                match chars[i] {
                                    '"' | '\\' | '$' | '`' | '\n' => {
                                        if chars[i] != '\n' {
                                            result.push(chars[i]);
                                        }
                                        i += 1;
                                    }
                                    _ => {
                                        result.push('\\');
                                    }
                                }
                            }
                        }
                        c => {
                            result.push(c);
                            i += 1;
                        }
                    }
                }
                if i < chars.len() { i += 1; } // closing "
            }
            '\\' => {
                i += 1;
                if i < chars.len() {
                    result.push(chars[i]);
                    i += 1;
                }
            }
            '$' => {
                let (expanded, consumed) = expand_dollar(&chars, i, vars, script_file);
                result.push_str(&expanded);
                i += consumed;
            }
            '`' => {
                let (expanded, consumed) = expand_backtick(&chars, i, vars, script_file);
                result.push_str(&expanded);
                i += consumed;
            }
            '~' => {
                // Tilde expansion at start or after :
                let tilde_result = expand_tilde(&chars, i, vars);
                result.push_str(&tilde_result.0);
                i += tilde_result.1;
            }
            '\'' => {
                // Single quotes - literal content
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    result.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() { i += 1; } // closing '
            }
            c => {
                result.push(c);
                i += 1;
            }
        }
    }
    result
}

fn expand_tilde(chars: &[char], start: usize, vars: &crate::shell::vars::VarStore) -> (String, usize) {
    let mut i = start + 1; // skip ~
    let mut name = String::new();
    while i < chars.len() && chars[i] != '/' && chars[i] != ':' && chars[i] != ' ' && chars[i] != '\t' {
        name.push(chars[i]);
        i += 1;
    }
    let consumed = i - start;

    if name.is_empty() {
        // ~ alone -> $HOME
        let home = vars.get_str("HOME").unwrap_or_else(|| {
            std::env::var("HOME").unwrap_or_else(|_| {
                // Try getpwuid
                get_home_dir_by_uid(unsafe { libc::getuid() })
            })
        });
        return (home, consumed);
    }

    // ~username
    match get_home_dir_by_name(&name) {
        Some(dir) => (dir, consumed),
        None => {
            // Keep literal
            let mut literal = String::from("~");
            literal.push_str(&name);
            (literal, consumed)
        }
    }
}

fn get_home_dir_by_name(username: &str) -> Option<String> {
    use std::ffi::CString;
    let cname = CString::new(username).ok()?;
    // SAFETY: getpwnam is called with a valid C string pointer
    let pw = unsafe { libc::getpwnam(cname.as_ptr()) };
    if pw.is_null() {
        return None;
    }
    // SAFETY: pw_dir is a valid C string if pw is non-null
    let dir = unsafe {
        std::ffi::CStr::from_ptr((*pw).pw_dir)
            .to_string_lossy()
            .into_owned()
    };
    Some(dir)
}

fn get_home_dir_by_uid(uid: u32) -> String {
    // SAFETY: getpwuid is called with a valid uid
    let pw = unsafe { libc::getpwuid(uid) };
    if pw.is_null() {
        return String::from("/");
    }
    // SAFETY: pw_dir is a valid C string if pw is non-null
    unsafe {
        std::ffi::CStr::from_ptr((*pw).pw_dir)
            .to_string_lossy()
            .into_owned()
    }
}

fn expand_dollar(chars: &[char], start: usize, vars: &crate::shell::vars::VarStore, script_file: &str) -> (String, usize) {
    let mut i = start + 1; // skip $
    if i >= chars.len() {
        return ("$".to_string(), 1);
    }

    match chars[i] {
        '{' => {
            // ${...}
            i += 1;
            let (result, consumed_from_open) = expand_brace(&chars[i..], vars, script_file);
            (result, 2 + consumed_from_open) // $ + { + content
        }
        '(' => {
            if i + 1 < chars.len() && chars[i+1] == '(' {
                // $(( arith ))
                i += 2;
                let (expr, consumed) = read_until_double_paren(&chars[i..]);
                let val = match eval_arith_expr_with_vars(&expr, vars) {
                    Ok(n) => n.to_string(),
                    Err(e) => {
                        eprintln!("zesh: {}", e);
                        PARAM_ERROR.with(|err| *err.borrow_mut() = true);
                        String::new()
                    }
                };
                (val, 3 + consumed) // $, (, (, content, ), )
            } else {
                // $( cmd )
                i += 1;
                let (body, consumed) = read_until_close_paren(&chars[i..]);
                let output = run_command_substitution(&body, vars, script_file);
                let output = output.trim_end_matches('\n').to_string();
                (output, 2 + consumed) // $, (, content, )
            }
        }
        '\'' => {
            // $'...' - already handled in lexer, but in case we see it here
            i += 1;
            let mut result = String::new();
            while i < chars.len() && chars[i] != '\'' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    match chars[i] {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '\'' => result.push('\''),
                        _ => { result.push('\\'); result.push(chars[i]); }
                    }
                } else {
                    result.push(chars[i]);
                }
                i += 1;
            }
            let consumed = i - start + 1; // +1 for closing '
            (result, consumed)
        }
        '@' | '*' => {
            let ch = chars[i];
            i += 1;
            // Get positional parameter count
            let count_str = vars.get_str("#").unwrap_or_else(|| "0".to_string());
            let count: usize = count_str.parse().unwrap_or(0);

            if count == 0 {
                (String::new(), i - start)
            } else {
                // Collect all positional parameters
                let mut params = Vec::new();
                for idx in 1..=count {
                    if let Some(val) = vars.get_str(&idx.to_string()) {
                        params.push(val);
                    }
                }

                // Determine separator
                let separator = if ch == '*' {
                    // $* uses first char of IFS
                    let ifs = vars.get_str("IFS").unwrap_or_else(|| " \t\n".to_string());
                    if ifs.is_empty() {
                        " ".to_string()
                    } else {
                        ifs.chars().next().unwrap().to_string()
                    }
                } else {
                    // $@ also uses first char of IFS for now (basic fix)
                    let ifs = vars.get_str("IFS").unwrap_or_else(|| " \t\n".to_string());
                    if ifs.is_empty() {
                        " ".to_string()
                    } else {
                        ifs.chars().next().unwrap().to_string()
                    }
                };

                let result = params.join(&separator);
                (result, i - start)
            }
        }
        '#' => {
            i += 1;
            if i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                // ${#VAR} - but this is $# case
                // Actually $# is parameter count
                let count = vars.get_str("#").unwrap_or_else(|| "0".to_string());
                (count, i - start)
            } else {
                let count = vars.get_str("#").unwrap_or_else(|| "0".to_string());
                (count, i - start)
            }
        }
        '?' => {
            i += 1;
            let status = vars.get_str("?").unwrap_or_else(|| "0".to_string());
            (status, i - start)
        }
        '$' => {
            i += 1;
            let pid = vars.get_str("$").unwrap_or_else(|| {
                #[cfg(feature = "fuzz")]
                { "99999".to_string() }
                #[cfg(not(feature = "fuzz"))]
                {
                    std::process::id().to_string()
                }
            });
            (pid, i - start)
        }
        '!' => {
            i += 1;
            let bg_pid = {
                #[cfg(feature = "fuzz")]
                { "99998".to_string() }
                #[cfg(not(feature = "fuzz"))]
                {
                    vars.get_str("!").unwrap_or_default()
                }
            };
            (bg_pid, i - start)
        }
        '0' => {
            i += 1;
            let v = vars.get_str("0").unwrap_or_default();
            (v, i - start)
        }
        '1'..='9' => {
            let n = (chars[i] as u8 - b'0') as usize;
            i += 1;
            let v = vars.get_str(&n.to_string()).unwrap_or_default();
            (v, i - start)
        }
        '_' | 'a'..='z' | 'A'..='Z' => {
            let mut name = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                name.push(chars[i]);
                i += 1;
            }
            let val = expand_special_var(&name, vars, script_file);
            (val, i - start)
        }
        _ => {
            ("$".to_string(), 1)
        }
    }
}

fn expand_special_var(name: &str, vars: &crate::shell::vars::VarStore, _script_file: &str) -> String {
    match name {
        "RANDOM" => {
            #[cfg(feature = "fuzz")]
            { "42".to_string() }
            #[cfg(not(feature = "fuzz"))]
            {
                let r = unsafe { libc::rand() } as u32 & 0x7fff;
                r.to_string()
            }
        }
        "SECONDS" => {
            #[cfg(feature = "fuzz")]
            { "42".to_string() }
            #[cfg(not(feature = "fuzz"))]
            {
                vars.get_str("SECONDS").unwrap_or_else(|| "0".to_string())
            }
        }
        "LINENO" => {
            vars.get_str("LINENO").unwrap_or_else(|| "0".to_string())
        }
        "FUNCNAME" => {
            vars.get_str("FUNCNAME").unwrap_or_default()
        }
        "BASH_SOURCE" => {
            vars.get_str("BASH_SOURCE").unwrap_or_default()
        }
        _ => {
            vars.get_str(name).unwrap_or_default()
        }
    }
}

fn expand_brace(chars: &[char], vars: &crate::shell::vars::VarStore, script_file: &str) -> (String, usize) {
    // Find matching }
    let mut depth = 1;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            '\'' => {
                i += 1;
                while i < chars.len() && chars[i] != '\'' { i += 1; }
            }
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    // Avoid skipping past end when backslash is the last char
                    if chars[i] == '\\' && i + 1 < chars.len() { i += 1; }
                    i += 1;
                }
            }
            _ => {}
        }
        // Guard: inner loops for ' and " can leave i at or past end
        if i >= chars.len() { break; }
        i += 1;
    }

    let i = i.min(chars.len()); // defensive cap in case of unterminated quote/brace
    let content: String = chars[..i].iter().collect();
    let consumed = i + 1; // include the }

    let result = expand_param(&content, vars, script_file);
    (result, consumed)
}

fn expand_param(content: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    // Handle special parameter expansions
    // ${#VAR} - length
    if content.starts_with('#') {
        let name = &content[1..];
        // Check for array ${#ARR[@]}
        if name.ends_with("[@]") || name.ends_with("[*]") {
            let arr_name = &name[..name.len()-3];
            let len = vars.array_len(arr_name);
            return len.to_string();
        }
        let val = expand_special_var(name, vars, script_file);
        return val.len().to_string();
    }

    // ${VAR@transform} — only when @ is not inside [...] and not part of $@ and not a special parameter
    if let Some(at_pos) = content.rfind('@') {
        let before = &content[..at_pos];
        // Skip if @ is inside brackets (e.g. ${ARR[@]})
        // Skip if @ is preceded by $ (e.g. ${1+$@} - here @ is part of $@, not a transform)
        // Skip if @ itself is the variable name (e.g. ${@+yes} - here @ is the special param)
        if !before.ends_with('[') && !before.ends_with('$') && !before.is_empty() {
            let transform = &content[at_pos+1..];
            let val = get_var_value(before, vars, script_file);
            return match transform {
                "U" => val.to_uppercase(),
                "L" | "l" => val.to_lowercase(),
                "Q" | "q" => format!("'{}'", val.replace('\'', "'\\''")),
                "E" => val, // TODO: process escape sequences
                _ => val,
            };
        }
    }

    // ${VAR:offset:length} - substring (or array slice for ${arr[@]:offset:length})
    if let Some(colon1) = find_colon_outside_parens(content) {
        let name = &content[..colon1];
        let rest = &content[colon1+1..];

        // Substring: :offset or :offset:length
        // Try to parse what comes next as a number (offset)
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if let Ok(offset) = parts[0].trim().parse::<i64>() {
            // ${arr[@]:offset} or ${arr[@]:offset:length} — slice element list
            let is_all = name.ends_with("[@]") || name.ends_with("[*]");
            if is_all {
                let arr_name = &name[..name.len()-3];
                if let Some(arr) = vars.get_array(arr_name) {
                    let mut keys: Vec<usize> = arr.keys().copied().collect();
                    keys.sort();
                    let elems: Vec<&str> = keys.iter().map(|k| arr[k].as_str()).collect();
                    let elen = elems.len() as i64;
                    let start = if offset < 0 {
                        (elen + offset).max(0) as usize
                    } else {
                        (offset as usize).min(elems.len())
                    };
                    if parts.len() > 1 {
                        if let Ok(length) = parts[1].trim().parse::<i64>() {
                            let end = if length < 0 {
                                (elen + length).max(0) as usize
                            } else {
                                (start as i64 + length).min(elen) as usize
                            };
                            let end = end.max(start);
                            return elems.get(start..end).map(|s| s.join(" ")).unwrap_or_default();
                        }
                    }
                    return elems.get(start..).map(|s| s.join(" ")).unwrap_or_default();
                }
                return String::new();
            }

            // Scalar substring
            let val = get_var_value(name, vars, script_file);
            let chars: Vec<char> = val.chars().collect();
            let slen = chars.len() as i64;
            let start = if offset < 0 {
                (slen + offset).max(0) as usize
            } else {
                (offset as usize).min(chars.len())
            };
            if parts.len() > 1 {
                if let Ok(length) = parts[1].trim().parse::<i64>() {
                    let end = if length < 0 {
                        (slen + length).max(0) as usize
                    } else {
                        (start as i64 + length).min(slen) as usize
                    };
                    let end = end.max(start);
                    if let Some(slice) = chars.get(start..end) {
                        return slice.iter().collect();
                    }
                    return String::new();
                }
            }
            if let Some(slice) = chars.get(start..) {
                return slice.iter().collect();
            }
            return String::new();
        } else if rest.starts_with('-') || rest.starts_with('=') || rest.starts_with('+') || rest.starts_with('?') {
            // Not a number, check for default/alternate operators
            return expand_default_param(name, rest, vars, script_file);
        }
    }

    // Check for ${VAR:-default} etc without colon
    if content.contains('-') || content.contains('=') || content.contains('+') || content.contains('?') {
        for (idx, c) in content.char_indices() {
            if matches!(c, '-' | '=' | '+' | '?') {
                let name = &content[..idx];
                let is_valid_name = is_valid_var_name(name) ||
                                    name.parse::<usize>().is_ok() ||  // numeric positional parameter
                                    name == "@" || name == "*" ||      // special parameters
                                    name == "#";                        // parameter count
                if is_valid_name {
                    let rest = &content[idx..];
                    return expand_default_param_no_colon(name, rest, vars, script_file);
                }
            }
        }
    }

    // Pattern removal ${VAR#pat}, ${VAR##pat}, ${VAR%pat}, ${VAR%%pat}
    // ${VAR/pat/repl}, ${VAR//pat/repl}
    // Process this BEFORE array indexing to handle ${arr[@]/pat/repl}
    for (i, c) in content.char_indices() {
        match c {
            '#' if i > 0 => {
                let name = &content[..i];
                let rest = &content[i+1..];
                let val = get_var_value(name, vars, script_file);
                let (greedy, pat) = if rest.starts_with('#') {
                    (true, &rest[1..])
                } else {
                    (false, rest)
                };
                return strip_prefix_pattern(&val, pat, greedy);
            }
            '%' if i > 0 => {
                let name = &content[..i];
                let rest = &content[i+1..];
                let val = get_var_value(name, vars, script_file);
                let (greedy, pat) = if rest.starts_with('%') {
                    (true, &rest[1..])
                } else {
                    (false, rest)
                };
                return strip_suffix_pattern(&val, pat, greedy);
            }
            '/' if i > 0 => {
                let name = &content[..i];
                let rest = &content[i+1..];
                let (global, rest) = if rest.starts_with('/') {
                    (true, &rest[1..])
                } else {
                    (false, rest)
                };
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                let pat = parts[0];
                let repl = if parts.len() > 1 { parts[1] } else { "" };

                // Check if this is array substitution ${arr[@]/pat/repl}
                if (name.ends_with("[@]") || name.ends_with("[*]")) && name.len() > 3 {
                    let arr_name = &name[..name.len()-3];
                    if let Some(arr) = vars.get_array(arr_name) {
                        let mut keys: Vec<usize> = arr.keys().copied().collect();
                        keys.sort();
                        // Expand variables in the replacement string
                        let expanded_repl = expand_string(repl, vars, script_file);
                        let replaced: Vec<String> = keys.iter()
                            .map(|k| replace_pattern(&arr[k], pat, &expanded_repl, global))
                            .collect();
                        return replaced.join(" ");
                    }
                    return String::new();
                }

                // Scalar substitution
                let val = get_var_value(name, vars, script_file);
                // Expand variables in the replacement string
                let expanded_repl = expand_string(repl, vars, script_file);
                return replace_pattern(&val, pat, &expanded_repl, global);
            }
            _ => {}
        }
    }

    // Array expansion ${ARR[idx]} or ${ARR[@]}
    if let Some(bracket_pos) = content.find('[') {
        let name = &content[..bracket_pos];
        let rest = &content[bracket_pos+1..];
        if let Some(end) = rest.find(']') {
            let idx_str = &rest[..end];
            if idx_str == "@" || idx_str == "*" {
                // Expand all elements
                if let Some(arr) = vars.get_array(name) {
                    let mut keys: Vec<usize> = arr.keys().copied().collect();
                    keys.sort();
                    return keys.iter().map(|k| arr[k].clone()).collect::<Vec<_>>().join(" ");
                }
                return String::new();
            }
            // Arithmetic index
            let idx = match eval_arith_expr_with_vars(idx_str, vars) {
                Ok(n) => n as usize,
                Err(_) => 0,
            };
            if let Some(arr) = vars.get_array(name) {
                return arr.get(&idx).cloned().unwrap_or_default();
            }
            return String::new();
        }
    }

    // Simple variable
    get_var_value(content, vars, script_file)
}

fn find_colon_outside_parens(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => { if depth > 0 { depth -= 1; } }
            ':' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn param_is_set(name: &str, vars: &crate::shell::vars::VarStore) -> bool {
    // Check if a parameter is set
    // For numeric positional parameters: check if index <= # count
    // For @ or *: check if # > 0
    // For named variables: check if it exists
    match name {
        "@" | "*" => {
            let count_str = vars.get_str("#").unwrap_or_else(|| "0".to_string());
            count_str.parse::<usize>().unwrap_or(0) > 0
        }
        _ => {
            // Check if it's a numeric positional parameter
            if let Ok(idx) = name.parse::<usize>() {
                if idx > 0 {
                    let count_str = vars.get_str("#").unwrap_or_else(|| "0".to_string());
                    let count = count_str.parse::<usize>().unwrap_or(0);
                    return idx <= count;
                }
            }
            // Named variable
            vars.get_str(name).is_some()
        }
    }
}

fn expand_default_param(name: &str, rest: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    // rest is the part after the colon: "-word", "=word", "+word", "?word"
    // OR it could be ":-word" if passed with the colon (legacy calling convention)
    let (op_char, word) = if rest.starts_with(":-") || rest.starts_with(":=") ||
                             rest.starts_with(":+") || rest.starts_with(":?") {
        // Has colon prefix
        (&rest[1..2], &rest[2..])
    } else if rest.starts_with('-') || rest.starts_with('=') ||
              rest.starts_with('+') || rest.starts_with('?') {
        (&rest[..1], &rest[1..])
    } else {
        return get_var_value(name, vars, script_file);
    };

    let val = get_var_value(name, vars, script_file);
    let is_set = param_is_set(name, vars);
    let is_unset_or_null = !is_set || val.is_empty();

    match op_char {
        "-" => {
            if is_unset_or_null {
                expand_string(word, vars, script_file)
            } else {
                val
            }
        }
        "=" => {
            if is_unset_or_null {
                let new_val = expand_string(word, vars, script_file);
                // Signal that we need to set this variable
                push_param_assign(name.to_string(), new_val.clone());
                new_val
            } else {
                val
            }
        }
        "+" => {
            if is_unset_or_null {
                String::new()
            } else {
                expand_string(word, vars, script_file)
            }
        }
        "?" => {
            if is_unset_or_null {
                let msg = expand_string(word, vars, script_file);
                eprintln!("zesh: {}: {}", name, msg);
                // Signal parameter error
                PARAM_ERROR.with(|e| *e.borrow_mut() = true);
                String::new()
            } else {
                val
            }
        }
        _ => val,
    }
}

fn expand_default_param_no_colon(name: &str, rest: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    let (op_char, word) = (&rest[..1], &rest[1..]);
    let is_set = param_is_set(name, vars);
    let val = get_var_value(name, vars, script_file);
    let is_unset = !is_set;

    match op_char {
        "-" => {
            if is_unset {
                expand_string(word, vars, script_file)
            } else {
                val
            }
        }
        "=" => {
            if is_unset {
                let new_val = expand_string(word, vars, script_file);
                push_param_assign(name.to_string(), new_val.clone());
                new_val
            } else {
                val
            }
        }
        "+" => {
            if is_unset {
                String::new()
            } else {
                expand_string(word, vars, script_file)
            }
        }
        "?" => {
            if is_unset {
                let msg = expand_string(word, vars, script_file);
                eprintln!("zesh: {}: {}", name, msg);
                PARAM_ERROR.with(|e| *e.borrow_mut() = true);
                String::new()
            } else {
                val
            }
        }
        _ => val,
    }
}

fn is_valid_var_name(s: &str) -> bool {
    if s.is_empty() { return false; }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' { return false; }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

fn get_var_value(name: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    // Returns the variable value, empty string if unset
    expand_special_var(name, vars, script_file)
}

fn is_var_unset(name: &str, vars: &crate::shell::vars::VarStore) -> bool {
    vars.get(name).is_none()
}

fn strip_prefix_pattern(val: &str, pat: &str, greedy: bool) -> String {
    if greedy {
        // Longest prefix match
        for end in (0..=val.len()).rev() {
            if let Some(slice) = val.get(..end) {
                if glob_match(pat, slice) {
                    return val[end..].to_string();
                }
            }
        }
    } else {
        // Shortest prefix match
        for end in 0..=val.len() {
            if let Some(slice) = val.get(..end) {
                if glob_match(pat, slice) {
                    return val[end..].to_string();
                }
            }
        }
    }
    val.to_string()
}

fn strip_suffix_pattern(val: &str, pat: &str, greedy: bool) -> String {
    if greedy {
        for start in 0..=val.len() {
            if let Some(slice) = val.get(start..) {
                if glob_match(pat, slice) {
                    return val[..start].to_string();
                }
            }
        }
    } else {
        for start in (0..=val.len()).rev() {
            if let Some(slice) = val.get(start..) {
                if glob_match(pat, slice) {
                    return val[..start].to_string();
                }
            }
        }
    }
    val.to_string()
}

fn replace_pattern(val: &str, pat: &str, repl: &str, global: bool) -> String {
    if global {
        // Replace all non-overlapping occurrences
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = val.chars().collect();
        while i <= chars.len() {
            let mut matched = false;
            for end in (i..=chars.len()).rev() {
                let slice: String = chars[i..end].iter().collect();
                if glob_match(pat, &slice) {
                    result.push_str(repl);
                    if end == i {
                        // Empty-pattern match: consume the next char too so we
                        // don't loop forever (mirrors bash "${v///R}" behaviour).
                        if i < chars.len() {
                            result.push(chars[i]);
                        }
                        i += 1;
                    } else {
                        i = end;
                    }
                    matched = true;
                    break;
                }
            }
            if !matched {
                if i < chars.len() {
                    result.push(chars[i]);
                }
                i += 1;
            }
        }
        result
    } else {
        // Replace first occurrence
        let chars: Vec<char> = val.chars().collect();
        for start in 0..=chars.len() {
            for end in (start..=chars.len()).rev() {
                let slice: String = chars[start..end].iter().collect();
                if glob_match(pat, &slice) {
                    let before: String = chars[..start].iter().collect();
                    let after: String = chars[end..].iter().collect();
                    return format!("{}{}{}", before, repl, after);
                }
            }
        }
        val.to_string()
    }
}

pub fn glob_match(pattern: &str, s: &str) -> bool {
    glob_match_chars(
        &pattern.chars().collect::<Vec<_>>(),
        0,
        &s.chars().collect::<Vec<_>>(),
        0,
    )
}

fn glob_match_chars(pat: &[char], pi: usize, s: &[char], si: usize) -> bool {
    if pi >= pat.len() {
        return si >= s.len();
    }

    // Extglob constructs: @( *( +( ?( !(
    if pi + 1 < pat.len() && matches!(pat[pi], '@' | '*' | '+' | '?' | '!') && pat[pi + 1] == '(' {
        let kind = pat[pi];
        if let Some(close) = extglob_find_close(pat, pi + 1) {
            let alts = extglob_split_alts(pat, pi + 2, close);
            let rest_pi = close + 1;
            return match kind {
                '@' => {
                    // Exactly one match of any alternative
                    for n in 0..=(s.len().saturating_sub(si)) {
                        let sub: Vec<char> = s[si..si + n].to_vec();
                        for alt in &alts {
                            if glob_match_chars(alt, 0, &sub, 0)
                                && glob_match_chars(pat, rest_pi, s, si + n)
                            {
                                return true;
                            }
                        }
                    }
                    false
                }
                '?' => {
                    // Zero or one match of any alternative
                    if glob_match_chars(pat, rest_pi, s, si) {
                        return true;
                    }
                    for n in 1..=(s.len().saturating_sub(si)) {
                        let sub: Vec<char> = s[si..si + n].to_vec();
                        for alt in &alts {
                            if glob_match_chars(alt, 0, &sub, 0)
                                && glob_match_chars(pat, rest_pi, s, si + n)
                            {
                                return true;
                            }
                        }
                    }
                    false
                }
                '*' => extglob_zero_or_more(&alts, rest_pi, pat, s, si),
                '+' => {
                    // One or more matches of any alternative
                    for n in 1..=(s.len().saturating_sub(si)) {
                        let sub: Vec<char> = s[si..si + n].to_vec();
                        for alt in &alts {
                            if glob_match_chars(alt, 0, &sub, 0) {
                                // After one mandatory match, allow zero or more more
                                if glob_match_chars(pat, rest_pi, s, si + n)
                                    || extglob_zero_or_more(&alts, rest_pi, pat, s, si + n)
                                {
                                    return true;
                                }
                            }
                        }
                    }
                    false
                }
                '!' => {
                    // Anything that doesn't match any alternative
                    for n in 0..=(s.len().saturating_sub(si)) {
                        let sub: Vec<char> = s[si..si + n].to_vec();
                        let matches_any = alts.iter().any(|alt| glob_match_chars(alt, 0, &sub, 0));
                        if !matches_any && glob_match_chars(pat, rest_pi, s, si + n) {
                            return true;
                        }
                    }
                    false
                }
                _ => false,
            };
        }
        // Malformed extglob (no matching ')') — fall through to literal match
    }

    match pat[pi] {
        '*' => {
            // Try matching zero or more characters
            for ni in si..=s.len() {
                if glob_match_chars(pat, pi + 1, s, ni) {
                    return true;
                }
            }
            false
        }
        '?' => {
            si < s.len() && glob_match_chars(pat, pi + 1, s, si + 1)
        }
        '[' => {
            // Character class - but only if it has a closing ]
            let mut pi2 = pi + 1;
            let negate = pi2 < pat.len() && pat[pi2] == '!';
            if negate { pi2 += 1; }
            let mut has_closing_bracket = false;
            let mut first = true;
            let mut test_pi2 = pi2;
            // First, check if there's a valid closing bracket
            while test_pi2 < pat.len() {
                if pat[test_pi2] == ']' && !first {
                    has_closing_bracket = true;
                    break;
                }
                first = false;
                test_pi2 += 1;
            }

            if has_closing_bracket {
                // Valid character class
                let mut matched = false;
                first = true;
                while pi2 < pat.len() && (first || pat[pi2] != ']') {
                    first = false;
                    // POSIX character class [:alpha:] etc.
                    if pi2 + 1 < pat.len() && pat[pi2] == '[' && pat[pi2 + 1] == ':' {
                        let class_start = pi2 + 2;
                        let mut class_end = class_start;
                        while class_end + 1 < pat.len()
                            && !(pat[class_end] == ':' && pat[class_end + 1] == ']')
                        {
                            class_end += 1;
                        }
                        if class_end + 1 < pat.len() {
                            let class_name: String = pat[class_start..class_end].iter().collect();
                            if si < s.len() {
                                let c = s[si];
                                if match class_name.as_str() {
                                    "alpha"  => c.is_alphabetic(),
                                    "digit"  => c.is_ascii_digit(),
                                    "alnum"  => c.is_alphanumeric(),
                                    "space"  => c.is_whitespace(),
                                    "upper"  => c.is_uppercase(),
                                    "lower"  => c.is_lowercase(),
                                    "print"  => !c.is_control(),
                                    "blank"  => c == ' ' || c == '\t',
                                    "punct"  => c.is_ascii_punctuation(),
                                    "cntrl"  => c.is_control(),
                                    "xdigit" => c.is_ascii_hexdigit(),
                                    _        => false,
                                } { matched = true; }
                            }
                            pi2 = class_end + 2; // skip past `:]`
                        } else {
                            // Malformed POSIX class — treat [ as literal
                            if si < s.len() && s[si] == '[' { matched = true; }
                            pi2 += 1;
                        }
                    } else if pi2 + 2 < pat.len() && pat[pi2 + 1] == '-' && pat[pi2 + 2] != ']' {
                        if si < s.len() && s[si] >= pat[pi2] && s[si] <= pat[pi2 + 2] {
                            matched = true;
                        }
                        pi2 += 3;
                    } else {
                        if si < s.len() && s[si] == pat[pi2] {
                            matched = true;
                        }
                        pi2 += 1;
                    }
                }
                if pi2 < pat.len() { pi2 += 1; } // skip ]
                let result = if negate { !matched } else { matched };
                result && si < s.len() && glob_match_chars(pat, pi2, s, si + 1)
            } else {
                // No closing bracket - treat '[' as literal
                si < s.len() && s[si] == '[' && glob_match_chars(pat, pi + 1, s, si + 1)
            }
        }
        '\\' if pi + 1 < pat.len() => {
            si < s.len() && s[si] == pat[pi + 1] && glob_match_chars(pat, pi + 2, s, si + 1)
        }
        c => {
            si < s.len() && s[si] == c && glob_match_chars(pat, pi + 1, s, si + 1)
        }
    }
}

fn extglob_find_close(pat: &[char], start: usize) -> Option<usize> {
    // pat[start] is '(' — find the matching ')'
    let mut depth = 1i32;
    let mut i = start + 1;
    while i < pat.len() {
        match pat[i] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extglob_split_alts(pat: &[char], start: usize, end: usize) -> Vec<Vec<char>> {
    // Split pat[start..end] by '|' at paren-depth 0
    let mut alts: Vec<Vec<char>> = Vec::new();
    let mut depth = 0i32;
    let mut seg_start = start;
    for i in start..end {
        match pat[i] {
            '(' => depth += 1,
            ')' => depth -= 1,
            '|' if depth == 0 => {
                alts.push(pat[seg_start..i].to_vec());
                seg_start = i + 1;
            }
            _ => {}
        }
    }
    alts.push(pat[seg_start..end].to_vec());
    alts
}

fn extglob_zero_or_more(alts: &[Vec<char>], rest_pi: usize, pat: &[char], s: &[char], si: usize) -> bool {
    // Match zero or more repetitions of any alternative, then rest_pi pattern
    if glob_match_chars(pat, rest_pi, s, si) {
        return true;
    }
    for n in 1..=(s.len().saturating_sub(si)) {
        let sub: Vec<char> = s[si..si + n].to_vec();
        for alt in alts {
            if glob_match_chars(alt, 0, &sub, 0)
                && extglob_zero_or_more(alts, rest_pi, pat, s, si + n)
            {
                return true;
            }
        }
    }
    false
}

fn expand_backtick(chars: &[char], start: usize, vars: &crate::shell::vars::VarStore, script_file: &str) -> (String, usize) {
    let mut i = start + 1; // skip `
    let mut body = String::new();
    while i < chars.len() {
        if chars[i] == '`' {
            i += 1;
            break;
        }
        if chars[i] == '\\' && i + 1 < chars.len() {
            if chars[i + 1] == '\n' {
                // \<newline> inside backtick = line continuation; discard both
                i += 2;
            } else {
                // Preserve the backslash and the next char; the subprocess shell
                // handles all other escape sequences in the correct quoting context.
                body.push('\\');
                body.push(chars[i + 1]);
                i += 2;
            }
        } else {
            body.push(chars[i]);
            i += 1;
        }
    }
    let output = run_command_substitution(&body, vars, script_file);
    let output = output.trim_end_matches('\n').to_string();
    (output, i - start)
}

fn read_until_double_paren(chars: &[char]) -> (String, usize) {
    let mut depth = 2; // we're inside ((
    let mut i = 0;
    let mut expr = String::new();
    while i < chars.len() {
        if chars[i] == ')' && i + 1 < chars.len() && chars[i+1] == ')' {
            depth -= 2;
            if depth <= 0 {
                i += 2;
                break;
            }
        } else if chars[i] == '(' {
            depth += 1;
            expr.push(chars[i]);
        } else if chars[i] == ')' {
            depth -= 1;
            expr.push(chars[i]);
        } else {
            expr.push(chars[i]);
        }
        i += 1;
    }
    (expr, i)
}

fn read_until_close_paren(chars: &[char]) -> (String, usize) {
    let mut depth = 1;
    let mut i = 0;
    let mut body = String::new();
    let mut word_buf = String::new();
    let mut case_depth = 0;
    let mut prev_keyword = String::new();

    while i < chars.len() {
        match chars[i] {
            '(' => {
                depth += 1;
                body.push('(');
                word_buf.clear();
            }
            ')' => {
                // Before checking depth, handle pending keywords
                if word_buf == "esac" && case_depth > 0 {
                    case_depth -= 1;
                    prev_keyword = "esac".to_string();
                }

                // Check if this ) closes a case pattern instead of a paren
                // A pattern close happens when: inside a case block, we have a word (or glob pattern), and we're expecting a pattern
                // This includes: after 'in' keyword, after ';;', or after another pattern
                let is_case_pattern_close = case_depth > 0 && !word_buf.is_empty() &&
                    (prev_keyword == "in" || prev_keyword == ";;" || prev_keyword == "pattern");

                if is_case_pattern_close {
                    // This ) closes a case pattern, not the $(...)
                    body.push(')');
                    word_buf.clear();
                    prev_keyword = "pattern".to_string();
                } else {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                    body.push(')');
                    word_buf.clear();
                    prev_keyword.clear();
                }
            }
            '\'' => {
                body.push('\'');
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    body.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() { body.push('\''); }
                word_buf.clear();
                prev_keyword.clear();
            }
            '"' => {
                body.push('"');
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' { body.push(chars[i]); i += 1; }
                    if i < chars.len() { body.push(chars[i]); }
                    i += 1;
                }
                if i < chars.len() { body.push('"'); }
                word_buf.clear();
                prev_keyword.clear();
            }
            ';' => {
                body.push(';');
                if i + 1 < chars.len() && chars[i + 1] == ';' {
                    i += 1;
                    body.push(';');
                    prev_keyword = ";;".to_string();
                }
                word_buf.clear();
            }
            c if c.is_whitespace() => {
                body.push(c);
                // Update prev_keyword based on word we just read
                if word_buf == "case" {
                    case_depth += 1;
                    prev_keyword = "case".to_string();
                } else if word_buf == "in" {
                    prev_keyword = "in".to_string();
                } else if word_buf == "esac" && case_depth > 0 {
                    case_depth -= 1;
                    prev_keyword = "esac".to_string();
                } else if !word_buf.is_empty() {
                    if prev_keyword == "in" || prev_keyword == ";;" || prev_keyword == "pattern" {
                        prev_keyword = "pattern".to_string();
                    } else {
                        prev_keyword.clear();
                    }
                }
                word_buf.clear();
            }
            c if c.is_alphanumeric() || c == '_' || c == '*' || c == '?' || c == '[' => {
                word_buf.push(c);
                body.push(c);
            }
            c => {
                body.push(c);
                word_buf.clear();
                prev_keyword.clear();
            }
        }
        i += 1;
    }
    (body, i)
}

pub fn run_command_substitution(cmd: &str, vars: &crate::shell::vars::VarStore, script_file: &str) -> String {
    use std::os::unix::io::FromRawFd;
    use std::io::Read;

    // Create pipe
    let mut pipe_fds = [0i32; 2];
    // SAFETY: pipe() is a valid syscall
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
        return String::new();
    }

    // SAFETY: fork() is a valid syscall
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        // SAFETY: closing valid file descriptors
        unsafe { libc::close(pipe_fds[0]); libc::close(pipe_fds[1]); }
        return String::new();
    }

    if pid == 0 {
        // Child
        // SAFETY: dup2 with valid fds
        unsafe {
            libc::close(pipe_fds[0]);
            libc::dup2(pipe_fds[1], 1);
            libc::close(pipe_fds[1]);
        }

        // Run the command
        let tokens = crate::shell::lexer::lex(cmd);
        let nodes = crate::shell::parser::parse(tokens);
        // Need to create a fresh context
        let mut ctx = crate::shell::executor::ExecContext::new_subshell();
        ctx.script_file = script_file.to_string();
        // Copy vars into subshell context
        // Actually we pass vars separately
        let status = crate::shell::executor::execute_list_with_vars(&nodes, &mut ctx, vars);
        // SAFETY: _exit is always safe to call
        unsafe { libc::_exit(status) };
    }

    // Parent
    // SAFETY: closing valid file descriptor
    unsafe { libc::close(pipe_fds[1]); }
    let mut file = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    let mut output = String::new();
    let _ = file.read_to_string(&mut output);

    // Wait for child
    let mut status = 0;
    // SAFETY: waitpid with valid pid
    unsafe { libc::waitpid(pid, &mut status, 0); }
    let exit_code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { 1 };
    LAST_CMDSUB_STATUS.with(|s| *s.borrow_mut() = Some(exit_code));

    output
}

pub fn eval_arith_expr_with_vars(expr: &str, vars: &crate::shell::vars::VarStore) -> Result<i64, String> {
    // Expand $VAR references in the expression, then evaluate
    // Store VarStore pointer for variable lookups (avoids expensive all_vars() clone)
    let depth_exceeded = EXPAND_DEPTH.with(|d| {
        let mut depth = d.borrow_mut();
        if *depth >= MAX_EXPAND_DEPTH {
            return true;
        }
        *depth += 1;
        false
    });
    if depth_exceeded {
        return Err("Expansion nesting too deep".to_string());
    }

    // Initialize ARITH_VARS for tracking in-arithmetic assignments, and set VarStore pointer
    ARITH_VARS.with(|v| {
        *v.borrow_mut() = Some(std::collections::HashMap::new());
    });
    ARITH_VARSTORE_PTR.with(|ptr| {
        *ptr.borrow_mut() = vars as *const _;
    });

    let expanded = expand_vars_in_arith(expr, vars);
    let result = eval_arith_expr(&expanded);

    // Clean up
    ARITH_VARS.with(|v| {
        *v.borrow_mut() = None;
    });
    ARITH_VARSTORE_PTR.with(|ptr| {
        *ptr.borrow_mut() = std::ptr::null();
    });

    EXPAND_DEPTH.with(|d| {
        *d.borrow_mut() -= 1;
    });
    result
}

fn expand_vars_in_arith(expr: &str, vars: &crate::shell::vars::VarStore) -> String {
    let chars: Vec<char> = expr.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' {
            i += 1;
            if i < chars.len() && chars[i] == '(' {
                // Nested $(())
                if i + 1 < chars.len() && chars[i+1] == '(' {
                    i += 2;
                    let (inner_expr, consumed) = read_until_double_paren(&chars[i..]);
                    let val = match eval_arith_expr_with_vars(&inner_expr, vars) {
                        Ok(n) => n.to_string(),
                        Err(e) => {
                            eprintln!("zesh: {}", e);
                            PARAM_ERROR.with(|err| *err.borrow_mut() = true);
                            "0".to_string()
                        }
                    };
                    result.push_str(&val);
                    i += consumed;
                } else {
                    result.push('$');
                }
            } else if i < chars.len() && chars[i].is_ascii_digit() {
                // Positional parameter: $1, $2, ..., $N
                let mut name = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    name.push(chars[i]);
                    i += 1;
                }
                let val = vars.get_str(&name).unwrap_or_default();
                let n: i64 = val.trim().parse().unwrap_or(0);
                result.push_str(&n.to_string());
            } else if i < chars.len() && (chars[i] == '*' || chars[i] == '@') {
                // $* and $@ — expand to space-joined positional params (raw, may be an expression)
                i += 1;
                let count_str = vars.get_str("#").unwrap_or_else(|| "0".to_string());
                let count: usize = count_str.parse().unwrap_or(0);
                let mut params = Vec::new();
                for idx in 1..=count {
                    params.push(vars.get_str(&idx.to_string()).unwrap_or_default());
                }
                result.push_str(&params.join(" "));
            } else if i < chars.len() && matches!(chars[i], '#' | '?' | '$' | '!') {
                // Special single-char variables
                let name = chars[i].to_string();
                i += 1;
                let val = vars.get_str(&name).unwrap_or_default();
                let n: i64 = val.trim().parse().unwrap_or(0);
                result.push_str(&n.to_string());
            } else if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
                let mut name = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    name.push(chars[i]);
                    i += 1;
                }
                let val = vars.get_str(&name).unwrap_or_default();
                let n: i64 = val.trim().parse().unwrap_or(0);
                result.push_str(&n.to_string());
            } else {
                result.push('$');
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

fn word_split(s: &str, ifs: &str) -> Vec<String> {
    if ifs.is_empty() {
        return vec![s.to_string()];
    }

    let ifs_whitespace: Vec<char> = ifs.chars().filter(|c| c.is_whitespace()).collect();
    let ifs_nonws: Vec<char> = ifs.chars().filter(|c| !c.is_whitespace()).collect();

    let mut parts = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if ifs_whitespace.contains(&c) {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
            // Skip whitespace
            while i < chars.len() && ifs_whitespace.contains(&chars[i]) {
                i += 1;
            }
        } else if ifs_nonws.contains(&c) {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            } else {
                parts.push(String::new());
            }
            i += 1;
            // Skip whitespace after non-whitespace IFS
            while i < chars.len() && ifs_whitespace.contains(&chars[i]) {
                i += 1;
            }
        } else {
            current.push(c);
            i += 1;
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn glob_expand(pattern: &str) -> Vec<String> {
    // Only glob if pattern contains glob chars
    if !pattern.contains('*') && !pattern.contains('?') && !pattern.contains('[') {
        return vec![pattern.to_string()];
    }

    match glob::glob(pattern) {
        Ok(paths) => {
            #[cfg(feature = "fuzz")]
            let mut results: Vec<String> = paths
                .filter_map(|p| p.ok())
                .map(|p| p.to_string_lossy().into_owned())
                .take(8)
                .collect();
            #[cfg(not(feature = "fuzz"))]
            let mut results: Vec<String> = paths
                .filter_map(|p| p.ok())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            if results.is_empty() {
                results.push(pattern.to_string());
            }
            results
        }
        Err(_) => vec![pattern.to_string()],
    }
}

// Expand a word token (from the lexer) - handles all the encoding
pub fn expand_token(tok: &crate::shell::types::Token, vars: &crate::shell::vars::VarStore, script_file: &str) -> Vec<String> {
    // Handle $'\n' etc - the lexer already processed these
    // The token value may contain raw chars if it was $'...' quoted

    // Check for process substitution
    if tok.value.starts_with("<(") || tok.value.starts_with(">(") {
        let is_input = tok.value.starts_with("<(");
        let end = if tok.value.ends_with(')') {
            tok.value.len() - 1
        } else {
            tok.value.len()
        };
        let cmd = if is_input { &tok.value[2..end] } else { &tok.value[2..end] };
        let path = create_process_substitution(cmd, vars, script_file, is_input);
        return vec![path];
    }

    let parts = expand_word(&tok.value, tok.quoted, vars, script_file);
    if parts.is_empty() && tok.quoted {
        vec![String::new()]
    } else {
        parts
    }
}

fn create_process_substitution(cmd: &str, vars: &crate::shell::vars::VarStore, script_file: &str, is_input: bool) -> String {
    let mut pipe_fds = [0i32; 2];
    // SAFETY: pipe() is a valid syscall
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
        return "/dev/null".to_string();
    }

    // SAFETY: fork() is a valid syscall
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        // SAFETY: closing valid file descriptors
        unsafe { libc::close(pipe_fds[0]); libc::close(pipe_fds[1]); }
        return "/dev/null".to_string();
    }

    if pid == 0 {
        // Child
        if is_input {
            // Output goes to pipe write end
            // SAFETY: dup2 with valid fds
            unsafe {
                libc::close(pipe_fds[0]);
                libc::dup2(pipe_fds[1], 1);
                libc::close(pipe_fds[1]);
            }
        } else {
            // SAFETY: dup2 with valid fds
            unsafe {
                libc::close(pipe_fds[1]);
                libc::dup2(pipe_fds[0], 0);
                libc::close(pipe_fds[0]);
            }
        }
        let tokens = crate::shell::lexer::lex(cmd);
        let nodes = crate::shell::parser::parse(tokens);
        let mut ctx = crate::shell::executor::ExecContext::new_subshell();
        ctx.script_file = script_file.to_string();
        let status = crate::shell::executor::execute_list_with_vars(&nodes, &mut ctx, vars);
        // SAFETY: _exit is always safe
        unsafe { libc::_exit(status) };
    }

    // Parent
    if is_input {
        // SAFETY: close valid fd
        unsafe { libc::close(pipe_fds[1]); }
        format!("/dev/fd/{}", pipe_fds[0])
    } else {
        // SAFETY: close valid fd
        unsafe { libc::close(pipe_fds[0]); }
        format!("/dev/fd/{}", pipe_fds[1])
    }
}

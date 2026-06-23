// Tokenizer for the shell binary

use crate::shell::types::{Token, TokKind};

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    pub line: u32,
}

impl Lexer {
    pub fn new(input: &str, start_line: u32) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: start_line,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        if c.is_some() {
            if c == Some('\n') {
                self.line += 1;
            }
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else if c == '\\' && self.peek2() == Some('\n') {
                // Line continuation
                self.advance(); // \
                self.advance(); // \n
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_single_quoted(&mut self) -> String {
        // Already consumed opening '
        let mut s = String::from("'");
        loop {
            match self.advance() {
                None => {
                    s.push('\'');
                    break;
                }
                Some('\'') => {
                    s.push('\'');
                    break;
                }
                Some(c) => s.push(c),
            }
        }
        s
    }

    fn read_dollar_single_quoted(&mut self) -> String {
        // $'\n' etc - already consumed $'
        let mut result = String::new();
        loop {
            match self.advance() {
                None => break,
                Some('\'') => break,
                Some('\\') => {
                    match self.advance() {
                        Some('n')  => result.push('\n'),
                        Some('t')  => result.push('\t'),
                        Some('r')  => result.push('\r'),
                        Some('a')  => result.push('\x07'),
                        Some('b')  => result.push('\x08'),
                        Some('f')  => result.push('\x0c'),
                        Some('v')  => result.push('\x0b'),
                        Some('e') | Some('E') => result.push('\x1b'),
                        Some('\\') => result.push('\\'),
                        Some('\'') => result.push('\''),
                        Some('"')  => result.push('"'),
                        Some('?')  => result.push('?'),
                        Some('x') => {
                            let mut hex = String::new();
                            for _ in 0..2 {
                                if let Some(h) = self.peek() {
                                    if h.is_ascii_hexdigit() {
                                        hex.push(h);
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            if !hex.is_empty() {
                                if let Ok(n) = u32::from_str_radix(&hex, 16) {
                                    if let Some(c) = char::from_u32(n) {
                                        result.push(c);
                                    }
                                }
                            }
                        }
                        Some('u') => {
                            let mut hex = String::new();
                            for _ in 0..4 {
                                if let Some(h) = self.peek() {
                                    if h.is_ascii_hexdigit() {
                                        hex.push(h);
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            if !hex.is_empty() {
                                if let Ok(n) = u32::from_str_radix(&hex, 16) {
                                    if let Some(c) = char::from_u32(n) {
                                        result.push(c);
                                    }
                                }
                            }
                        }
                        Some(c) if c >= '0' && c <= '7' => {
                            let mut oct = String::new();
                            oct.push(c);
                            for _ in 0..2 {
                                if let Some(h) = self.peek() {
                                    if h >= '0' && h <= '7' {
                                        oct.push(h);
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            if let Ok(n) = u32::from_str_radix(&oct, 8) {
                                if let Some(c) = char::from_u32(n) {
                                    result.push(c);
                                }
                            }
                        }
                        Some(c) => {
                            result.push('\\');
                            result.push(c);
                        }
                        None => break,
                    }
                }
                Some(c) => result.push(c),
            }
        }
        result
    }

    fn read_double_quoted(&mut self) -> String {
        // Already consumed opening "
        // Returns content, with nested expansions preserved as-is
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some(nc @ ('"' | '\\' | '$' | '`' | '\n')) => {
                            let nc = nc;
                            self.advance();
                            if nc != '\n' {
                                s.push('\\');
                                s.push(nc);
                            }
                        }
                        _ => {
                            s.push('\\');
                        }
                    }
                }
                Some('$') => {
                    // Preserve $ expansions for expand_string to handle
                    s.push(self.advance().unwrap());
                    // Read the expansion content
                    if let Some(next) = self.peek() {
                        match next {
                            '{' => {
                                s.push(self.advance().unwrap());
                                let body = self.read_brace_expansion();
                                s.push_str(&body);
                                s.push('}');
                            }
                            '(' => {
                                s.push(self.advance().unwrap());
                                if self.peek() == Some('(') {
                                    s.push(self.advance().unwrap());
                                    let body = self.read_double_paren();
                                    s.push_str(&body);
                                    s.push_str("))");
                                } else {
                                    let body = self.read_paren_body();
                                    s.push_str(&body);
                                    s.push(')');
                                }
                            }
                            '\'' => {
                                // $'...' inside double quotes
                                self.advance();
                                let escaped = self.read_dollar_single_quoted();
                                s.push_str(&escaped);
                            }
                            _ => {
                                // $VAR or $# etc - read identifier
                                let rest = self.read_var_name_in_dq();
                                s.push_str(&rest);
                            }
                        }
                    }
                }
                Some('`') => {
                    s.push(self.advance().unwrap());
                    let body = self.read_backtick_body();
                    s.push_str(&body);
                    s.push('`');
                }
                Some(_) => {
                    s.push(self.advance().unwrap());
                }
            }
        }
        s
    }

    fn read_var_name_in_dq(&mut self) -> String {
        let mut s = String::new();
        // Special chars
        if let Some(c) = self.peek() {
            match c {
                '@' | '*' | '#' | '?' | '$' | '!' | '0'..='9' | '_' | 'a'..='z' | 'A'..='Z' => {
                    s.push(self.advance().unwrap());
                    if c.is_alphanumeric() || c == '_' {
                        while let Some(nc) = self.peek() {
                            if nc.is_alphanumeric() || nc == '_' {
                                s.push(self.advance().unwrap());
                            } else {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        s
    }

    fn read_brace_expansion(&mut self) -> String {
        // Read until matching }
        let mut s = String::new();
        let mut depth = 1;
        loop {
            match self.peek() {
                None => break,
                Some('{') => {
                    depth += 1;
                    s.push(self.advance().unwrap());
                }
                Some('}') => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance(); // consume }
                        break;
                    }
                    s.push(self.advance().unwrap());
                }
                Some('\'') => {
                    s.push(self.advance().unwrap());
                    loop {
                        match self.advance() {
                            None | Some('\'') => { s.push('\''); break; }
                            Some(c) => s.push(c),
                        }
                    }
                }
                Some('"') => {
                    s.push(self.advance().unwrap());
                    loop {
                        match self.advance() {
                            None | Some('"') => { s.push('"'); break; }
                            Some('\\') => { s.push('\\'); if let Some(c) = self.advance() { s.push(c); } }
                            Some(c) => s.push(c),
                        }
                    }
                }
                Some(_) => {
                    s.push(self.advance().unwrap());
                }
            }
        }
        s
    }

    fn read_paren_body(&mut self) -> String {
        let mut s = String::new();
        let mut depth = 1;
        let mut case_depth = 0;  // Track how many case...esac blocks we're in
        let mut word_buf = String::new();
        let mut prev_keyword = String::new();  // Track the last keyword seen
        let mut bracket_depth = 0;  // Track bracket expression [...] nesting

        loop {
            match self.peek() {
                None => break,
                Some('[') if bracket_depth == 0 => {
                    // Starting a bracket expression
                    bracket_depth += 1;
                    word_buf.push('[');
                    s.push(self.advance().unwrap());
                }
                Some(']') if bracket_depth > 0 => {
                    // Ending a bracket expression
                    bracket_depth -= 1;
                    word_buf.push(']');
                    s.push(self.advance().unwrap());
                }
                Some(c) if bracket_depth > 0 => {
                    // Inside bracket expression, treat all chars as part of word
                    word_buf.push(c);
                    s.push(self.advance().unwrap());
                }
                Some('(') => {
                    depth += 1;
                    s.push(self.advance().unwrap());
                    word_buf.clear();
                }
                Some(')') => {
                    // First, handle pending keywords that haven't been processed yet
                    if word_buf == "esac" && case_depth > 0 {
                        case_depth -= 1;
                        prev_keyword = "esac".to_string();
                    }

                    // A bare ) in a case block closes a pattern, not a paren
                    // This happens when: case_depth > 0 AND we just read a word AND we're in a pattern context
                    let is_case_pattern_close = case_depth > 0 && !word_buf.is_empty() &&
                        (prev_keyword == "in" || prev_keyword == ";;" || prev_keyword == "pattern");

                    if is_case_pattern_close {
                        // This ) closes a case pattern, not the $(...) substitution
                        s.push(self.advance().unwrap());
                        word_buf.clear();
                        prev_keyword = "pattern".to_string();  // Mark that we just closed a pattern
                    } else {
                        // This ) closes the substitution or a nested paren
                        depth -= 1;
                        if depth == 0 {
                            self.advance(); // consume )
                            break;
                        }
                        s.push(self.advance().unwrap());
                        word_buf.clear();
                        prev_keyword.clear();
                    }
                }
                Some('\'') => {
                    s.push(self.advance().unwrap());
                    loop {
                        match self.advance() {
                            None | Some('\'') => { s.push('\''); break; }
                            Some(c) => s.push(c),
                        }
                    }
                    word_buf.clear();
                    prev_keyword.clear();
                }
                Some('"') => {
                    s.push(self.advance().unwrap());
                    loop {
                        match self.advance() {
                            None | Some('"') => { s.push('"'); break; }
                            Some('\\') => { s.push('\\'); if let Some(c) = self.advance() { s.push(c); } }
                            Some(c) => s.push(c),
                        }
                    }
                    word_buf.clear();
                    prev_keyword.clear();
                }
                Some('\\') => {
                    s.push(self.advance().unwrap());
                    if let Some(c) = self.advance() { s.push(c); }
                    word_buf.clear();
                    prev_keyword.clear();
                }
                Some(';') => {
                    s.push(self.advance().unwrap());
                    if self.peek() == Some(';') {
                        s.push(self.advance().unwrap());
                        // After ;;, expect next pattern
                        prev_keyword = ";;".to_string();
                    }
                    word_buf.clear();
                }
                Some(c) if c.is_whitespace() => {
                    s.push(self.advance().unwrap());
                    // Update prev_keyword based on the word we just read
                    if word_buf == "case" {
                        case_depth += 1;
                        prev_keyword = "case".to_string();
                    } else if word_buf == "in" {
                        prev_keyword = "in".to_string();
                    } else if word_buf == "esac" && case_depth > 0 {
                        case_depth -= 1;
                        prev_keyword = "esac".to_string();
                    } else if !word_buf.is_empty() {
                        // Any other word - might be a pattern or regular word
                        if prev_keyword == "in" || prev_keyword == ";;" || prev_keyword == "pattern" {
                            prev_keyword = "pattern".to_string();
                        } else {
                            prev_keyword.clear();
                        }
                    }
                    word_buf.clear();
                }
                Some(c) if c.is_alphanumeric() || c == '_' || c == '*' || c == '?' => {
                    word_buf.push(c);
                    s.push(self.advance().unwrap());
                }
                Some(c) => {
                    s.push(self.advance().unwrap());
                    word_buf.clear();
                    prev_keyword.clear();
                }
            }
        }
        s
    }

    fn read_double_paren(&mut self) -> String {
        // Read until matching ))
        let mut s = String::new();
        let mut depth = 2;
        loop {
            match self.peek() {
                None => break,
                Some('(') => { depth += 1; s.push(self.advance().unwrap()); }
                Some(')') => {
                    if depth == 2 {
                        // Check for ))
                        if self.input.get(self.pos + 1) == Some(&')') {
                            self.advance(); // first )
                            self.advance(); // second )
                            break;
                        }
                    }
                    depth -= 1;
                    s.push(self.advance().unwrap());
                }
                Some(_) => { s.push(self.advance().unwrap()); }
            }
        }
        s
    }

    fn read_backtick_body(&mut self) -> String {
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('`') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    s.push(self.advance().unwrap());
                    if let Some(c) = self.advance() { s.push(c); }
                }
                Some(_) => { s.push(self.advance().unwrap()); }
            }
        }
        s
    }

    fn read_word_raw(&mut self) -> (String, bool) {
        // Read a word token, handling quoting. Returns (value, quoted)
        let mut value = String::new();
        let mut quoted = false;
        // Track if we're inside a \${...} sequence (literal braces after escaped $)
        let mut in_escaped_brace = false;
        let mut bracket_depth = 0;  // Track bracket expression [...] nesting

        loop {
            match self.peek() {
                None => break,
                // IMPORTANT: Handle quotes/dollar/escape BEFORE checking is_word_break/bracket_depth
                // This ensures proper processing of constructs like "${arr[0]}"
                Some('\'') => {
                    quoted = true;
                    self.advance();
                    let sq = self.read_single_quoted();
                    value.push_str(&sq);
                }
                Some('$') => {
                    // Check for $'...' ANSI-C quoting
                    if self.peek2() == Some('\'') {
                        self.advance(); // $
                        self.advance(); // '
                        let s = self.read_dollar_single_quoted();
                        quoted = true; // treat as quoted (no word splitting)
                        value.push_str(&s);
                    } else {
                        // Keep $ and following expansion content
                        value.push(self.advance().unwrap()); // $
                        match self.peek() {
                            Some('{') => {
                                value.push(self.advance().unwrap()); // {
                                let body = self.read_brace_expansion();
                                value.push_str(&body);
                                value.push('}');
                            }
                            Some('(') => {
                                value.push(self.advance().unwrap()); // (
                                if self.peek() == Some('(') {
                                    value.push(self.advance().unwrap()); // second (
                                    let body = self.read_double_paren();
                                    value.push_str(&body);
                                    value.push_str("))");
                                } else {
                                    let body = self.read_paren_body();
                                    value.push_str(&body);
                                    value.push(')');
                                }
                            }
                            _ => {
                                // $VAR, $#, $$, etc
                                // read until word break
                                while let Some(c) = self.peek() {
                                    if c.is_alphanumeric() || c == '_' ||
                                       (value == "$" && matches!(c, '@' | '*' | '#' | '?' | '!' | '0'..='9')) {
                                        value.push(self.advance().unwrap());
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Some('"') => {
                    quoted = true;
                    self.advance(); // consume "
                    let dq = self.read_double_quoted();
                    // Wrap in double-quote markers
                    value.push('"');
                    value.push_str(&dq);
                    value.push('"');
                }
                Some('`') => {
                    self.advance();
                    let bt = self.read_backtick_body();
                    value.push('`');
                    value.push_str(&bt);
                    value.push('`');
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('\n') => {
                            self.advance(); // line continuation
                        }
                        Some(c) => {
                            let c = c;
                            self.advance();
                            value.push('\\');
                            value.push(c);
                        }
                        None => {}
                    }
                }
                // Bracket expressions: handle [ and ] for glob patterns
                // These must come AFTER quote/expansion handling so [0] inside "${x[0]}" works
                // IMPORTANT: Don't enter bracket mode if [ is followed by whitespace,
                // since "[ ... ]" is the test command, not a glob bracket expression
                Some('[') if bracket_depth == 0 && !matches!(self.peek2(), Some(' ') | Some('\t') | Some('\n') | None) => {
                    // Starting a bracket expression (only if next char is not whitespace)
                    bracket_depth += 1;
                    value.push('[');
                    self.advance();
                }
                Some(']') if bracket_depth > 0 => {
                    // Ending a bracket expression
                    bracket_depth -= 1;
                    value.push(']');
                    self.advance();
                }
                Some(c) if bracket_depth > 0 => {
                    // Inside bracket expression, all chars are literal (don't break on whitespace)
                    value.push(c);
                    self.advance();
                }
                // Word break handling
                Some(c) if is_word_break(c) => {
                    // Don't stop at { if preceded by \$ (escaped dollar sign)
                    if c == '{' && value.ends_with("\\$") {
                        // Start tracking that we're in a literal ${...}
                        value.push(c);
                        self.advance();
                        in_escaped_brace = true;
                        continue;
                    }
                    // Don't stop at } if we're inside a \${...} sequence
                    if c == '}' && in_escaped_brace {
                        value.push(c);
                        self.advance();
                        in_escaped_brace = false;
                        continue;
                    }
                    break;
                }
                // Default: regular character
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }
        (value, quoted)
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            let line = self.line;

            match self.peek() {
                None => {
                    tokens.push(Token { kind: TokKind::Eof, value: String::new(), quoted: false, line });
                    break;
                }
                Some('#') => {
                    self.skip_comment();
                }
                Some('\n') => {
                    self.advance();
                    tokens.push(Token { kind: TokKind::Newline, value: "\n".to_string(), quoted: false, line });
                }
                Some(';') => {
                    self.advance();
                    if self.peek() == Some(';') {
                        self.advance();
                        tokens.push(Token { kind: TokKind::Semi, value: ";;".to_string(), quoted: false, line });
                    } else {
                        tokens.push(Token { kind: TokKind::Semi, value: ";".to_string(), quoted: false, line });
                    }
                }
                Some('|') => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        tokens.push(Token { kind: TokKind::Or, value: "||".to_string(), quoted: false, line });
                    } else if self.peek() == Some('&') {
                        self.advance();
                        tokens.push(Token { kind: TokKind::PipeErr, value: "|&".to_string(), quoted: false, line });
                    } else {
                        tokens.push(Token { kind: TokKind::Pipe, value: "|".to_string(), quoted: false, line });
                    }
                }
                Some('&') => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        tokens.push(Token { kind: TokKind::And, value: "&&".to_string(), quoted: false, line });
                    } else {
                        tokens.push(Token { kind: TokKind::Bg, value: "&".to_string(), quoted: false, line });
                    }
                }
                Some('(') => {
                    self.advance();
                    tokens.push(Token { kind: TokKind::LParen, value: "(".to_string(), quoted: false, line });
                }
                Some(')') => {
                    self.advance();
                    tokens.push(Token { kind: TokKind::RParen, value: ")".to_string(), quoted: false, line });
                }
                Some('!') => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push(Token { kind: TokKind::Word, value: "!=".to_string(), quoted: false, line });
                    } else {
                        tokens.push(Token { kind: TokKind::Bang, value: "!".to_string(), quoted: false, line });
                    }
                }
                Some('<') => {
                    self.advance();
                    if self.peek() == Some('<') {
                        self.advance();
                        if self.peek() == Some('<') {
                            self.advance();
                            tokens.push(Token { kind: TokKind::RedirHerestr, value: "<<<".to_string(), quoted: false, line });
                        } else if self.peek() == Some('-') {
                            self.advance();
                            tokens.push(Token { kind: TokKind::RedirHeredoc, value: "<<-".to_string(), quoted: false, line });
                        } else {
                            tokens.push(Token { kind: TokKind::RedirHeredoc, value: "<<".to_string(), quoted: false, line });
                        }
                    } else if self.peek() == Some('&') {
                        self.advance();
                        // <& - read target
                        self.skip_whitespace();
                        let target = self.read_redir_target();
                        let val = format!("0&{}", target);
                        tokens.push(Token { kind: TokKind::RedirDupIn, value: val, quoted: false, line });
                    } else if self.peek() == Some('(') {
                        self.advance();
                        let body = self.read_proc_subst();
                        tokens.push(Token { kind: TokKind::Word, value: format!("<({body})"), quoted: false, line });
                    } else {
                        tokens.push(Token { kind: TokKind::RedirIn, value: "<".to_string(), quoted: false, line });
                    }
                }
                Some('>') => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        tokens.push(Token { kind: TokKind::RedirAppend, value: ">>".to_string(), quoted: false, line });
                    } else if self.peek() == Some('&') {
                        self.advance();
                        // >& - read target
                        self.skip_whitespace();
                        let target = self.read_redir_target();
                        let val = format!("1&{}", target);
                        tokens.push(Token { kind: TokKind::RedirDupOut, value: val, quoted: false, line });
                    } else if self.peek() == Some('(') {
                        self.advance();
                        let body = self.read_proc_subst();
                        tokens.push(Token { kind: TokKind::Word, value: format!(">({body})"), quoted: false, line });
                    } else {
                        tokens.push(Token { kind: TokKind::RedirOut, value: ">".to_string(), quoted: false, line });
                    }
                }
                Some(c) if c.is_ascii_digit() => {
                    // May be N> or N< or N>> or N>& or N<& or just a number
                    let mut num = String::new();
                    while let Some(d) = self.peek() {
                        if d.is_ascii_digit() {
                            num.push(d);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    match self.peek() {
                        Some('>') => {
                            self.advance();
                            if self.peek() == Some('>') {
                                self.advance();
                                tokens.push(Token { kind: TokKind::RedirFdAppend, value: num, quoted: false, line });
                            } else if self.peek() == Some('&') {
                                self.advance();
                                // Read target
                                let target = self.read_redir_target();
                                let val = if target.is_empty() {
                                    // No inline target - will be next token
                                    format!("{}", num)
                                } else {
                                    format!("{}&{}", num, target)
                                };
                                tokens.push(Token { kind: TokKind::RedirDupOut, value: val, quoted: false, line });
                            } else {
                                tokens.push(Token { kind: TokKind::RedirFdOut, value: num, quoted: false, line });
                            }
                        }
                        Some('<') => {
                            self.advance();
                            if self.peek() == Some('&') {
                                self.advance();
                                let target = self.read_redir_target();
                                let val = if target.is_empty() {
                                    format!("{}", num)
                                } else {
                                    format!("{}&{}", num, target)
                                };
                                tokens.push(Token { kind: TokKind::RedirDupIn, value: val, quoted: false, line });
                            } else {
                                tokens.push(Token { kind: TokKind::RedirFdIn, value: num, quoted: false, line });
                            }
                        }
                        _ => {
                            // Just a word starting with digits - continue reading
                            let (rest, quoted) = self.read_word_raw();
                            let full = format!("{}{}", num, rest);
                            let kind = word_to_keyword(&full);
                            tokens.push(Token { kind, value: full, quoted, line });
                        }
                    }
                }
                _ => {
                    // Read a word
                    let (value, quoted) = self.read_word_raw();
                    if value.is_empty() {
                        self.advance(); // Safety: skip unknown char
                        continue;
                    }
                    let kind = word_to_keyword(&value);
                    tokens.push(Token { kind, value, quoted, line });
                }
            }
        }

        // Now process heredocs
        self.process_heredocs(&mut tokens);

        tokens
    }

    fn read_proc_subst(&mut self) -> String {
        let mut depth = 1;
        let mut body = String::new();
        loop {
            match self.advance() {
                None => break,
                Some('(') => {
                    depth += 1;
                    body.push('(');
                }
                Some(')') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    body.push(')');
                }
                Some('\\') => {
                    body.push('\\');
                    if let Some(c) = self.advance() {
                        body.push(c);
                    }
                }
                Some('\'') => {
                    body.push('\'');
                    loop {
                        match self.advance() {
                            None | Some('\'') => {
                                body.push('\'');
                                break;
                            }
                            Some(c) => body.push(c),
                        }
                    }
                }
                Some('"') => {
                    body.push('"');
                    loop {
                        match self.advance() {
                            None | Some('"') => {
                                body.push('"');
                                break;
                            }
                            Some('\\') => {
                                body.push('\\');
                                if let Some(c) = self.advance() { body.push(c); }
                            }
                            Some(c) => body.push(c),
                        }
                    }
                }
                Some(c) => body.push(c),
            }
        }
        body
    }

    fn read_redir_target(&mut self) -> String {
        let mut s = String::new();
        if self.peek() == Some('-') {
            s.push('-');
            self.advance();
        } else {
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }
        s
    }

    fn process_heredocs(&mut self, tokens: &mut Vec<Token>) {
        let mut i = 0;
        let mut next_body_start: u32 = 0;
        let mut last_redir_line: u32 = 0;

        while i < tokens.len() {
            if tokens[i].kind == TokKind::RedirHeredoc {
                if i + 1 < tokens.len() {
                    let delim_raw = tokens[i + 1].value.clone();
                    let strip_tabs = tokens[i].value == "<<-";
                    let delim_line = tokens[i + 1].line;

                    // For multiple heredocs on the same command line, bodies follow each other.
                    let start_line = if delim_line == last_redir_line && next_body_start > 0 {
                        next_body_start
                    } else {
                        delim_line + 1
                    };

                    let bare_delim: String = delim_raw.chars()
                        .filter(|&c| c != '\'' && c != '"' && c != '\\')
                        .collect();
                    let bare_delim = bare_delim.trim().to_string();

                    let (content, lines_consumed) =
                        self.collect_heredoc_from_line(start_line, &bare_delim, strip_tabs);

                    tokens[i + 1].value = format!("\x00HEREDOC\x00{}\x00{}", delim_raw, content);

                    if lines_consumed > 0 {
                        let last_body_line = start_line + lines_consumed - 1;
                        last_redir_line = delim_line;
                        next_body_start = last_body_line + 1;

                        // Remove tokens that belong to the heredoc body and closing delimiter.
                        // These are all tokens whose line is in [start_line, last_body_line].
                        // Tokens at indices <= i+1 are on lines <= delim_line < start_line and are kept.
                        let s = start_line;
                        let e = last_body_line;
                        tokens.retain(|tok| tok.line < s || tok.line > e);
                    }
                }
            }
            i += 1;
        }
    }

    // Return the start byte-index in self.input of the given 1-based line number.
    fn find_line_start(&self, line: u32) -> usize {
        if line <= 1 {
            return 0;
        }
        let mut current = 1u32;
        for (idx, &c) in self.input.iter().enumerate() {
            if c == '\n' {
                current += 1;
                if current == line {
                    return idx + 1;
                }
            }
        }
        self.input.len()
    }

    // Collect heredoc body starting at `start_line` in self.input.
    // Returns (content, lines_consumed) where lines_consumed includes the closing delimiter line.
    fn collect_heredoc_from_line(&self, start_line: u32, bare_delim: &str, strip_tabs: bool) -> (String, u32) {
        let start_pos = self.find_line_start(start_line);
        if start_pos >= self.input.len() {
            return (String::new(), 0);
        }
        let remaining_str: String = self.input[start_pos..].iter().collect();
        if remaining_str.is_empty() {
            return (String::new(), 0);
        }

        let mut content = String::new();
        let mut lines_consumed = 0u32;

        for line in remaining_str.lines() {
            lines_consumed += 1;
            let check = if strip_tabs { line.trim_start_matches('\t') } else { line };
            if check == bare_delim {
                break;
            }
            let actual = if strip_tabs { line.trim_start_matches('\t') } else { line };
            content.push_str(actual);
            content.push('\n');
        }

        (content, lines_consumed)
    }
}

fn is_word_break(c: char) -> bool {
    // These characters end a word when encountered unquoted
    // Note: { and } are NOT word breaks — they can appear as literal characters in words
    // (e.g., echo {} or find ... -exec ... {} \;). They should only be special tokens
    // when the parser recognizes { as starting a compound statement (which happens via
    // word-value inspection, not token-kind inspection).
    matches!(c, ' ' | '\t' | '\n' | ';' | '&' | '|' | '<' | '>' | '(' | ')')
}

fn word_to_keyword(word: &str) -> TokKind {
    match word {
        "if"       => TokKind::If,
        "then"     => TokKind::Then,
        "else"     => TokKind::Else,
        "elif"     => TokKind::Elif,
        "fi"       => TokKind::Fi,
        "while"    => TokKind::While,
        "until"    => TokKind::Until,
        "do"       => TokKind::Do,
        "done"     => TokKind::Done,
        "for"      => TokKind::For,
        "in"       => TokKind::In,
        "case"     => TokKind::Case,
        "esac"     => TokKind::Esac,
        "select"   => TokKind::Select,
        "function" => TokKind::Function,
        "time"     => TokKind::Time,
        "coproc"   => TokKind::Coproc,
        _          => TokKind::Word,
    }
}

pub fn lex(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input, 1);
    lexer.tokenize()
}

pub fn lex_with_line(input: &str, start_line: u32) -> Vec<Token> {
    let mut lexer = Lexer::new(input, start_line);
    lexer.tokenize()
}

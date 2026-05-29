use serde_json::{Number, Value};

use crate::nodes::{CompareOp, Condition, ForLoop, If, Include, LocalValue, Node, Operand};

pub fn parse_template(input: &str) -> Vec<Node> {
    let mut p = Parser::new(input);
    p.parse_nodes(None)
}

struct Parser<'a> {
    src: &'a str,
    byte_offset: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            byte_offset: 0,
        }
    }

    fn parse_defer(&mut self) -> Node {
        self.byte_offset += "@defer".len();

        self.skip_whitespace();
        self.expect_char('(');

        let inner = self.read_until_unbalanced(')', '(');
        let split_at = find_top_level_char(&inner, ';');
        let path = split_at
            .map(|i| inner[..i].trim().to_string())
            .unwrap_or_else(|| inner.trim().to_string());
        let local_ctx = split_at
            .map(|i| parse_kv_pairs(&inner[i + 1..]))
            .unwrap_or_default();

        self.skip_whitespace();
        let body = if self.peek_char() == Some('{') {
            self.byte_offset += 1;
            self.parse_nodes(Some('}'))
        } else {
            Vec::new()
        };

        Node::Include(Include {
            path,
            body,
            local_ctx,
        })
    }

    fn parse_nodes(&mut self, end_on: Option<char>) -> Vec<Node> {
        let mut nodes = Vec::new();
        let mut text_buf = String::new();

        while !self.eof() {
            if let Some(end) = end_on
                && self.peek_char() == Some(end)
            {
                if !text_buf.is_empty() {
                    nodes.push(Node::Text(std::mem::take(&mut text_buf)));
                }
                self.byte_offset += end.len_utf8();
                break;
            }

            if self.starts_with("{{") {
                if !text_buf.is_empty() {
                    nodes.push(Node::Text(std::mem::take(&mut text_buf)));
                }
                nodes.push(Node::VariableBlock(self.parse_variable()));
                continue;
            }

            if self.starts_with("@defer") {
                if !text_buf.is_empty() {
                    nodes.push(Node::Text(std::mem::take(&mut text_buf)));
                }
                nodes.push(self.parse_defer());
                continue;
            }

            if self.starts_with("@if") {
                if !text_buf.is_empty() {
                    nodes.push(Node::Text(std::mem::take(&mut text_buf)));
                }
                nodes.push(self.parse_if());
                continue;
            }

            if self.starts_with("@for") {
                if !text_buf.is_empty() {
                    nodes.push(Node::Text(std::mem::take(&mut text_buf)));
                }
                nodes.push(self.parse_for());
                continue;
            }

            if self.starts_with("@else") {
                if !text_buf.is_empty() {
                    nodes.push(Node::Text(std::mem::take(&mut text_buf)));
                }
                text_buf.push_str("@else");
                self.byte_offset += "@else".len();
                continue;
            }

            if let Some(ch) = self.peek_char() {
                text_buf.push(ch);
                self.advance_one();
            } else {
                break;
            }
        }

        if !text_buf.is_empty() {
            nodes.push(Node::Text(text_buf));
        }

        nodes
    }

    fn parse_variable(&mut self) -> Vec<String> {
        self.byte_offset += 2;
        let start = self.byte_offset;

        while !self.eof() {
            if self.starts_with("}}") {
                let expr = self.src[start..self.byte_offset].trim();
                self.byte_offset += 2;
                if expr == "content" {
                    return vec!["__CONTENT__".to_string()];
                }
                return parse_variable_path(expr);
            }
            self.advance_one();
        }

        parse_variable_path(self.src[start..].trim())
    }

    fn parse_if(&mut self) -> Node {
        self.byte_offset += "@if".len();

        self.skip_whitespace();
        self.expect_char('(');

        let expr = self.read_until_unbalanced(')', '(');
        let cond = parse_bool_expr(expr.trim());

        self.skip_whitespace();
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        let mut conditions = vec![(cond, body)];
        let mut otherwise: Option<Vec<Node>> = None;

        loop {
            let save = self.byte_offset;

            self.skip_whitespace();

            if self.starts_with("@else") {
                self.byte_offset += "@else".len();
                self.skip_whitespace();

                if self.starts_with("if") {
                    self.byte_offset += "if".len();
                    self.skip_whitespace();
                    self.expect_char('(');
                    let expr = self.read_until_unbalanced(')', '(');
                    let cond = parse_bool_expr(expr.trim());

                    self.skip_whitespace();
                    self.expect_char('{');
                    let body = self.parse_nodes(Some('}'));

                    conditions.push((cond, body));
                    continue;
                }

                self.skip_whitespace();
                self.expect_char('{');
                otherwise = Some(self.parse_nodes(Some('}')));
                break;
            }

            self.byte_offset = save;
            break;
        }

        Node::If(If {
            conditions,
            otherwise,
        })
    }

    fn parse_for(&mut self) -> Node {
        self.byte_offset += "@for".len();

        self.skip_whitespace();
        self.expect_char('(');

        let for_expr = self.read_until_unbalanced(')', '(');
        let (value, container_str) = parse_for_expression(&for_expr);
        let container = parse_variable_path(container_str.trim());

        self.skip_whitespace();
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        Node::Forloop(ForLoop {
            value,
            container,
            body,
        })
    }

    fn read_until_unbalanced(&mut self, end: char, start_pair: char) -> String {
        let start_position = self.byte_offset;
        let mut depth = 0usize;
        let mut quote: Option<char> = None;
        let mut escaped = false;

        for (i, c) in self.src[self.byte_offset..].char_indices() {
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == q {
                    quote = None;
                }
                continue;
            }

            if c == '"' || c == '\'' {
                quote = Some(c);
                continue;
            }

            if c == start_pair {
                depth += 1;
            } else if c == end {
                if depth == 0 {
                    let end_byte = self.byte_offset + i;
                    let s = self.src[start_position..end_byte].to_string();
                    self.byte_offset = end_byte + end.len_utf8();
                    return s;
                }
                depth -= 1;
            }
        }

        let s = self.src[start_position..].to_string();
        self.byte_offset = self.src.len();
        s
    }

    #[inline]
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.byte_offset += c.len_utf8();
            } else {
                break;
            }
        }
    }

    #[inline]
    fn expect_char(&mut self, expected: char) {
        self.skip_whitespace();
        if self.peek_char() == Some(expected) {
            self.byte_offset += expected.len_utf8();
        }
    }

    #[inline]
    fn peek_char(&self) -> Option<char> {
        self.src[self.byte_offset..].chars().next()
    }

    #[inline]
    fn advance_one(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.byte_offset += ch.len_utf8();
        }
    }

    #[inline]
    fn eof(&self) -> bool {
        self.byte_offset >= self.src.len()
    }

    #[inline]
    fn starts_with(&self, s: &str) -> bool {
        self.src[self.byte_offset..].starts_with(s)
    }
}

fn parse_for_expression(expr: &str) -> (String, String) {
    let trimmed = expr.trim();

    if let Some(i) = find_top_level_keyword(trimmed, "in") {
        let value = trimmed[..i].trim().to_string();
        let container = trimmed[i + "in".len()..].trim().to_string();
        return (value, container);
    }

    (trimmed.to_string(), String::new())
}

fn parse_kv_pairs(s: &str) -> Vec<(String, LocalValue)> {
    let mut pairs = Vec::new();
    let mut i = 0;

    while i < s.len() {
        i = skip_kv_separators(s, i);

        if i >= s.len() {
            break;
        }

        let key_start = i;

        while i < s.len() {
            let ch = char_at(s, i).unwrap();
            if ch == '=' || ch.is_whitespace() || ch == ';' || ch == ',' {
                break;
            }
            i += ch.len_utf8();
        }

        let key = s[key_start..i].trim();

        i = skip_spaces(s, i);

        if key.is_empty() || char_at(s, i) != Some('=') {
            while i < s.len() {
                let ch = char_at(s, i).unwrap();
                i += ch.len_utf8();
                if ch.is_whitespace() || ch == ';' || ch == ',' {
                    break;
                }
            }
            continue;
        }

        i += 1;
        i = skip_spaces(s, i);

        let (value, next_i) = read_kv_value(s, i);
        i = next_i;

        pairs.push((key.to_string(), parse_local_value(value.trim())));
    }

    pairs
}

fn parse_local_value(v: &str) -> LocalValue {
    if let Some(s) = unquote(v) {
        return LocalValue::Literal(Value::String(s));
    }

    if v == "true" {
        return LocalValue::Literal(Value::Bool(true));
    }

    if v == "false" {
        return LocalValue::Literal(Value::Bool(false));
    }

    if v == "null" {
        return LocalValue::Literal(Value::Null);
    }

    if let Ok(i) = v.parse::<i64>() {
        return LocalValue::Literal(Value::Number(i.into()));
    }

    if let Ok(f) = v.parse::<f64>()
        && let Some(n) = Number::from_f64(f)
    {
        return LocalValue::Literal(Value::Number(n));
    }

    LocalValue::Path(parse_variable_path(v))
}

fn read_kv_value(s: &str, mut i: usize) -> (&str, usize) {
    let start = i;

    if let Some(q @ ('"' | '\'')) = char_at(s, i) {
        i += q.len_utf8();
        let mut escaped = false;

        while i < s.len() {
            let ch = char_at(s, i).unwrap();
            i += ch.len_utf8();

            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == q {
                break;
            }
        }

        return (&s[start..i], i);
    }

    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;

    while i < s.len() {
        let ch = char_at(s, i).unwrap();

        if let Some(q) = quote {
            i += ch.len_utf8();

            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == q {
                quote = None;
            }

            continue;
        }

        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                i += ch.len_utf8();
            }
            '[' => {
                bracket_depth += 1;
                i += ch.len_utf8();
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                i += ch.len_utf8();
            }
            '(' => {
                paren_depth += 1;
                i += ch.len_utf8();
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                i += ch.len_utf8();
            }
            ';' | ',' if bracket_depth == 0 && paren_depth == 0 => break,
            c if c.is_whitespace() && bracket_depth == 0 && paren_depth == 0 => break,
            _ => i += ch.len_utf8(),
        }
    }

    (&s[start..i], i)
}

fn skip_kv_separators(s: &str, mut i: usize) -> usize {
    while i < s.len() {
        let ch = char_at(s, i).unwrap();
        if ch.is_whitespace() || ch == ';' || ch == ',' {
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    i
}

fn skip_spaces(s: &str, mut i: usize) -> usize {
    while i < s.len() {
        let ch = char_at(s, i).unwrap();
        if ch.is_whitespace() {
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    i
}

fn parse_variable_path(expr: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_brackets = false;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for c in expr.trim().chars() {
        if let Some(q) = quote {
            if escaped {
                current.push(c);
                escaped = false;
                continue;
            }

            if c == '\\' {
                escaped = true;
                continue;
            }

            if c == q {
                quote = None;
                continue;
            }

            current.push(c);
            continue;
        }

        match c {
            '"' | '\'' if in_brackets => quote = Some(c),
            '[' => {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                in_brackets = true;
            }
            ']' => {
                if in_brackets {
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                    }
                    current.clear();
                    in_brackets = false;
                }
            }
            '.' if !in_brackets => {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
            }
            c if c.is_whitespace() && !in_brackets => {}
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

#[derive(Debug, Clone)]
enum Token {
    Ident(String),
    And,
    Or,
    Not,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    LParen,
    RParen,
}

struct Cursor {
    tokens: Vec<Token>,
    position: usize,
}

impl Cursor {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.position).cloned();
        if t.is_some() {
            self.position += 1;
        }
        t
    }
}

fn parse_bool_expr(s: &str) -> Condition {
    let tokens = tokenize_bool(s);
    let mut cur = Cursor::new(tokens);
    parse_expr(&mut cur)
}

fn tokenize_bool(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                cur.push(c);

                let quote = c;
                let mut escaped = false;

                for ch in chars.by_ref() {
                    cur.push(ch);

                    if escaped {
                        escaped = false;
                        continue;
                    }

                    if ch == '\\' {
                        escaped = true;
                        continue;
                    }

                    if ch == quote {
                        break;
                    }
                }
            }
            '(' => {
                push_ident_token(&mut cur, &mut tokens);
                tokens.push(Token::LParen);
            }
            ')' => {
                push_ident_token(&mut cur, &mut tokens);
                tokens.push(Token::RParen);
            }
            '=' => {
                if chars.peek() == Some(&'=') {
                    push_ident_token(&mut cur, &mut tokens);
                    chars.next();
                    tokens.push(Token::Eq);
                } else {
                    cur.push(c);
                }
            }
            '!' => {
                push_ident_token(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ne);
                } else {
                    tokens.push(Token::Not);
                }
            }
            '<' => {
                push_ident_token(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Le);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                push_ident_token(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ge);
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '&' => {
                push_ident_token(&mut cur, &mut tokens);
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::And);
                } else {
                    cur.push(c);
                }
            }
            '|' => {
                push_ident_token(&mut cur, &mut tokens);
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    cur.push(c);
                }
            }
            c if c.is_whitespace() => push_ident_token(&mut cur, &mut tokens),
            _ => cur.push(c),
        }
    }

    push_ident_token(&mut cur, &mut tokens);
    tokens
}

fn push_ident_token(cur: &mut String, tokens: &mut Vec<Token>) {
    if cur.is_empty() {
        return;
    }

    let w = cur.trim().to_string();
    cur.clear();

    match w.as_str() {
        "and" => tokens.push(Token::And),
        "or" => tokens.push(Token::Or),
        "not" => tokens.push(Token::Not),
        _ if !w.is_empty() => tokens.push(Token::Ident(w)),
        _ => {}
    }
}

fn parse_expr(cur: &mut Cursor) -> Condition {
    let left = parse_term(cur);
    let mut parts = vec![left];

    while let Some(Token::Or) = cur.peek() {
        cur.next();
        parts.push(parse_term(cur));
    }

    if parts.len() == 1 {
        parts[0].clone()
    } else {
        Condition::Or(parts)
    }
}

fn parse_term(cur: &mut Cursor) -> Condition {
    let left = parse_unary(cur);
    let mut parts = vec![left];

    while let Some(Token::And) = cur.peek() {
        cur.next();
        parts.push(parse_unary(cur));
    }

    if parts.len() == 1 {
        parts[0].clone()
    } else {
        Condition::And(parts)
    }
}

fn parse_unary(cur: &mut Cursor) -> Condition {
    match cur.peek() {
        Some(Token::Not) => {
            cur.next();
            Condition::Not(Box::new(parse_unary(cur)))
        }
        _ => parse_factor(cur),
    }
}

fn parse_factor(cur: &mut Cursor) -> Condition {
    match cur.peek() {
        Some(Token::LParen) => {
            cur.next();
            let inner = parse_expr(cur);
            if let Some(Token::RParen) = cur.peek() {
                cur.next();
            }
            inner
        }
        Some(Token::Ident(_)) => {
            let left_ident = if let Some(Token::Ident(s)) = cur.next() {
                s
            } else {
                String::new()
            };

            if let Some(op_tok) = cur.peek()
                && let Some(op) = parse_compare_op(op_tok)
            {
                cur.next();
                let right = parse_operand(cur.next());
                let left = operand_from_expr(&left_ident);
                return Condition::Compare { left, op, right };
            }

            if is_literal_expr(&left_ident) {
                Condition::Literal(
                    parse_literal(Some(Token::Ident(left_ident)))
                        .as_bool()
                        .unwrap_or_default(),
                )
            } else {
                Condition::Path(parse_variable_path(&left_ident))
            }
        }
        _ => Condition::Literal(false),
    }
}

fn parse_operand(tok: Option<Token>) -> Operand {
    match tok {
        Some(Token::Ident(s)) => operand_from_expr(&s),
        other => Operand::Literal(parse_literal(other)),
    }
}

fn operand_from_expr(s: &str) -> Operand {
    if is_literal_expr(s) {
        Operand::Literal(parse_literal(Some(Token::Ident(s.to_string()))))
    } else {
        Operand::Path(parse_variable_path(s))
    }
}

fn is_literal_expr(s: &str) -> bool {
    s == "true"
        || s == "false"
        || s == "null"
        || unquote(s).is_some()
        || s.parse::<i64>().is_ok()
        || s.parse::<f64>().is_ok()
}

fn parse_compare_op(tok: &Token) -> Option<CompareOp> {
    match tok {
        Token::Eq => Some(CompareOp::Eq),
        Token::Ne => Some(CompareOp::Ne),
        Token::Lt => Some(CompareOp::Lt),
        Token::Gt => Some(CompareOp::Gt),
        Token::Le => Some(CompareOp::Le),
        Token::Ge => Some(CompareOp::Ge),
        _ => None,
    }
}

fn parse_literal(tok: Option<Token>) -> Value {
    match tok {
        Some(Token::Ident(s)) => {
            if s == "true" {
                Value::Bool(true)
            } else if s == "false" {
                Value::Bool(false)
            } else if s == "null" {
                Value::Null
            } else if let Ok(i) = s.parse::<i64>() {
                Value::Number(i.into())
            } else if let Ok(f) = s.parse::<f64>() {
                Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else if let Some(unquoted) = unquote(&s) {
                Value::String(unquoted)
            } else {
                Value::String(s)
            }
        }
        _ => Value::Null,
    }
}

fn unquote(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let quote = chars.next()?;

    if quote != '"' && quote != '\'' {
        return None;
    }

    if !s.ends_with(quote) || s.len() < quote.len_utf8() * 2 {
        return None;
    }

    let inner = &s[quote.len_utf8()..s.len() - quote.len_utf8()];
    let mut out = String::new();
    let mut escaped = false;

    for ch in inner.chars() {
        if escaped {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '\'' => out.push('\''),
                other => out.push(other),
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            out.push(ch);
        }
    }

    if escaped {
        out.push('\\');
    }

    Some(out)
}

fn find_top_level_char(s: &str, target: char) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for (i, ch) in s.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == q {
                quote = None;
            }

            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            c if c == target && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(i);
            }
            _ => {}
        }
    }

    None
}

fn find_top_level_keyword(s: &str, keyword: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for (i, ch) in s.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == q {
                quote = None;
            }

            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {
                if paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0
                    && s[i..].starts_with(keyword)
                    && is_keyword_boundary(s, i, keyword.len())
                {
                    return Some(i);
                }
            }
        }
    }

    None
}

fn is_keyword_boundary(s: &str, start: usize, len: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        s[..start].chars().next_back()
    };

    let after_index = start + len;
    let after = if after_index >= s.len() {
        None
    } else {
        s[after_index..].chars().next()
    };

    before.map(|c| c.is_whitespace()).unwrap_or(true)
        && after.map(|c| c.is_whitespace()).unwrap_or(true)
}

fn char_at(s: &str, i: usize) -> Option<char> {
    s.get(i..)?.chars().next()
}

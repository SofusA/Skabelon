use serde_json::Value;

use crate::nodes::{CompareOp, Condition, ForLoop, If, Include, LocalValue, Node, Operand};

pub fn parse_template(input: &str) -> Vec<Node> {
    let mut p = Parser::new(input);
    p.parse_nodes(None)
}

struct Parser<'a> {
    src: &'a str,
    chars: Vec<char>,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.chars().collect(),
            position: 0,
        }
    }

    fn parse_include(&mut self) -> Node {
        self.position += "@include".len();

        self.skip_ws();
        self.expect_char('(');

        let inner = self.read_until_unbalanced(')', '(');
        let mut parts = inner.splitn(2, ';').map(|s| s.trim());
        let path = parts.next().unwrap_or("").to_string();
        let local_ctx = parts.next().map(parse_kv_pairs).unwrap_or_default();

        // Optional block `{ ... }`
        self.skip_ws();
        let body = if self.peek_char() == Some('{') {
            self.position += 1; // consume '{'
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
                    if !text_buf.is_empty() {
                        nodes.push(Node::Text(text_buf.clone()));
                    }

                    text_buf.clear();
                }
                self.position += 1;
                break;
            }

            if self.starts_with("{{") {
                if !text_buf.is_empty() {
                    if !text_buf.is_empty() {
                        nodes.push(Node::Text(text_buf.clone()));
                    }
                    text_buf.clear();
                }
                nodes.push(Node::VariableBlock(self.parse_variable()));
                continue;
            }

            if self.starts_with("@include") {
                if !text_buf.is_empty() {
                    if !text_buf.is_empty() {
                        nodes.push(Node::Text(text_buf.clone()));
                    }
                    text_buf.clear();
                }
                nodes.push(self.parse_include());
                continue;
            }

            if self.starts_with("@if") {
                if !text_buf.is_empty() {
                    if !text_buf.is_empty() {
                        nodes.push(Node::Text(text_buf.clone()));
                    }
                    text_buf.clear();
                }
                nodes.push(self.parse_if());
                continue;
            }

            if self.starts_with("@for") {
                if !text_buf.is_empty() {
                    if !text_buf.is_empty() {
                        nodes.push(Node::Text(text_buf.clone()));
                    }
                    text_buf.clear();
                }
                nodes.push(self.parse_for());
                continue;
            }

            if self.starts_with("@else") {
                if !text_buf.is_empty() {
                    if !text_buf.is_empty() {
                        nodes.push(Node::Text(text_buf.clone()));
                    }
                    text_buf.clear();
                }

                text_buf.push_str("@else");
                self.position += "@else".len();
                continue;
            }

            text_buf.push(self.chars[self.position]);
            self.position += 1;
        }

        if !text_buf.is_empty() && !text_buf.is_empty() {
            nodes.push(Node::Text(text_buf.clone()));
        }

        nodes
    }

    fn parse_variable(&mut self) -> Vec<String> {
        debug_assert!(self.starts_with("{{"));
        self.position += 2;
        let start = self.position;

        while !self.eof() {
            if self.starts_with("}}") {
                let expr = self.src[start..self.position].trim();
                self.position += 2;

                let trimmed = expr.trim();
                if trimmed == "content" {
                    return vec!["__CONTENT__".to_string()];
                }
                return parse_variable_path(trimmed);
            }
            self.position += 1;
        }

        parse_variable_path(self.src[start..].trim())
    }

    fn parse_if(&mut self) -> Node {
        self.position += "@if".len();

        self.skip_ws();
        self.expect_char('(');

        let expr = self.read_until_unbalanced(')', '(');
        let cond = parse_bool_expr(expr.trim());

        self.skip_ws();
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        let mut conditions = Vec::new();
        conditions.push((cond, body));
        let mut otherwise: Option<Vec<Node>> = None;

        loop {
            self.skip_ws();

            if self.starts_with("@else") {
                self.position += "@else".len();
                self.skip_ws();

                if self.starts_with("if") {
                    // '@else if (...) { ... }'
                    self.position += "if".len();
                    self.skip_ws();
                    self.expect_char('(');
                    let expr = self.read_until_unbalanced(')', '(');
                    let cond = parse_bool_expr(expr.trim());

                    self.skip_ws();
                    self.expect_char('{');
                    let body = self.parse_nodes(Some('}'));

                    conditions.push((cond, body));
                    continue;
                } else {
                    // '@else { ... }'
                    self.skip_ws();
                    self.expect_char('{');
                    let else_body = self.parse_nodes(Some('}'));
                    otherwise = Some(else_body);
                    break;
                }
            } else {
                break;
            }
        }

        Node::If(If {
            conditions,
            otherwise,
        })
    }

    fn parse_for(&mut self) -> Node {
        self.position += "@for".len();

        self.skip_ws();
        self.expect_char('(');

        let for_expr = self.read_until_unbalanced(')', '(');
        let (value, container_str) = parse_for_expression(&for_expr);

        let container = parse_variable_path(container_str.trim());

        self.skip_ws();
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        Node::Forloop(ForLoop {
            value,
            container,
            body,
        })
    }

    fn read_until_unbalanced(&mut self, end: char, start_pair: char) -> String {
        let start_position = self.position;
        let mut depth = 0;

        while !self.eof() {
            let c = self.chars[self.position];

            if c == start_pair {
                depth += 1;
            } else if c == end {
                if depth == 0 {
                    let s = self.src[start_position..self.position].to_string();
                    self.position += 1;
                    return s;
                } else {
                    depth -= 1;
                }
            }
            self.position += 1;
        }

        self.src[start_position..].to_string()
    }

    fn expect_char(&mut self, expected: char) {
        self.skip_ws();
        if self.peek_char() == Some(expected) {
            self.position += 1;
        }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn eof(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src.get(self.position..self.position + s.len()) == Some(s)
    }
}

fn parse_for_expression(expr: &str) -> (String, String) {
    let trimmed = expr.trim();
    let mut parts = trimmed.splitn(2, " in ");
    let value = parts.next().unwrap_or("").trim().to_string();
    let container = parts.next().unwrap_or("").trim().to_string();
    (value, container)
}

fn parse_kv_pairs(s: &str) -> Vec<(String, LocalValue)> {
    let normalized = s.replace(';', " ");
    let tokens = normalized.split_whitespace();

    tokens
        .filter_map(|pair| {
            let mut kv = pair.splitn(2, '=').map(|x| x.trim());
            let k = kv.next()?;
            let v = kv.next()?;

            if (v.starts_with('"') && v.ends_with('"'))
                || (v.starts_with('\'') && v.ends_with('\''))
            {
                let inner = &v[1..v.len() - 1];
                return Some((
                    k.to_string(),
                    LocalValue::Literal(Value::String(inner.to_string())),
                ));
            }

            if v == "true" {
                return Some((k.to_string(), LocalValue::Literal(Value::Bool(true))));
            }

            if v == "false" {
                return Some((k.to_string(), LocalValue::Literal(Value::Bool(false))));
            }

            if v == "null" {
                return Some((k.to_string(), LocalValue::Literal(Value::Null)));
            }

            if let Ok(i) = v.parse::<i64>() {
                return Some((k.to_string(), LocalValue::Literal(Value::Number(i.into()))));
            }

            if let Ok(f) = v.parse::<f64>() {
                return Some((k.to_string(), LocalValue::Literal(serde_json::json!(f))));
            }

            Some((k.to_string(), LocalValue::Path(parse_variable_path(v))))
        })
        .collect()
}

fn parse_variable_path(expr: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_brackets = false;

    for c in expr.chars() {
        match c {
            '[' => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
                in_brackets = true;
            }
            ']' => {
                if in_brackets {
                    parts.push(current.clone());
                    current.clear();
                    in_brackets = false;
                }
            }
            '"' | '\'' => continue,
            '.' if !in_brackets => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        parts.push(current);
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

fn parse_unary(cur: &mut Cursor) -> Condition {
    match cur.peek() {
        Some(Token::Not) => {
            cur.next(); // consume '!'
            let inner = parse_unary(cur); // right-associative
            Condition::Not(Box::new(inner))
        }
        _ => parse_factor(cur),
    }
}

fn tokenize_bool(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cur = String::new();

    let push_cur = |cur: &mut String, tokens: &mut Vec<Token>| {
        if cur.is_empty() {
            return;
        }
        let w = cur.trim().to_string();
        cur.clear();
        match w.as_str() {
            "and" | "&&" => tokens.push(Token::And),
            "or" | "||" => tokens.push(Token::Or),
            "not" => tokens.push(Token::Not),
            _ => tokens.push(Token::Ident(w)),
        }
    };

    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' => {
                push_cur(&mut cur, &mut tokens);
                tokens.push(Token::LParen);
            }
            ')' => {
                push_cur(&mut cur, &mut tokens);
                tokens.push(Token::RParen);
            }
            '=' => {
                push_cur(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Eq);
                }
            }
            '!' => {
                push_cur(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ne);
                } else {
                    tokens.push(Token::Not);
                }
            }
            '<' => {
                push_cur(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Le);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                push_cur(&mut cur, &mut tokens);
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ge);
                } else {
                    tokens.push(Token::Gt);
                }
            }

            c if c.is_whitespace() => {
                push_cur(&mut cur, &mut tokens);
            }
            _ => cur.push(c),
        }
    }
    push_cur(&mut cur, &mut tokens);
    tokens
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

fn parse_expr(cur: &mut Cursor) -> Condition {
    let left = parse_term(cur);
    let mut parts = vec![left];

    while let Some(Token::Or) = cur.peek() {
        cur.next();
        let rhs = parse_term(cur);
        parts.push(rhs);
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
        let rhs = parse_unary(cur);
        parts.push(rhs);
    }

    if parts.len() == 1 {
        parts[0].clone()
    } else {
        Condition::And(parts)
    }
}

fn parse_factor(cur: &mut Cursor) -> Condition {
    match cur.peek() {
        Some(Token::LParen) => {
            cur.next(); // '('
            let inner = parse_expr(cur);
            if let Some(Token::RParen) = cur.peek() {
                cur.next(); // ')'
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
                cur.next(); // consume operator
                let right = parse_operand(cur.next());
                let left = Operand::Path(parse_variable_path(&left_ident));
                return Condition::Compare { left, op, right };
            }
            Condition::Path(parse_variable_path(&left_ident))
        }
        _ => Condition::Literal(false),
    }
}

fn parse_operand(tok: Option<Token>) -> Operand {
    match tok {
        Some(Token::Ident(s)) => {
            let t = s.as_str();
            let is_quoted = (t.starts_with('"') && t.ends_with('"'))
                || (t.starts_with('\'') && t.ends_with('\''));
            let is_bool = t == "true" || t == "false";
            let is_int = t.parse::<i64>().is_ok();
            let is_float = t.parse::<f64>().is_ok();

            if is_quoted || is_bool || is_int || is_float {
                Operand::Literal(parse_literal(Some(Token::Ident(s))))
            } else {
                Operand::Path(parse_variable_path(&s))
            }
        }
        other => Operand::Literal(parse_literal(other)),
    }
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
            } else if let Ok(i) = s.parse::<i64>() {
                Value::Number(i.into())
            } else if let Ok(f) = s.parse::<f64>() {
                serde_json::json!(f)
            } else if (s.starts_with('"') && s.ends_with('"'))
                || (s.starts_with('\'') && s.ends_with('\''))
            {
                Value::String(s[1..s.len() - 1].to_string())
            } else {
                Value::String(s)
            }
        }
        _ => Value::Null,
    }
}

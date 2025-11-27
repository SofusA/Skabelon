use crate::nodes::{Condition, ForLoop, If, Include, LocalValue, Node};

pub fn parse_template(input: &str) -> Vec<Node> {
    let mut p = Parser::new(input);
    p.parse_nodes(None)
}

struct Parser<'a> {
    src: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.chars().collect(),
            pos: 0,
        }
    }

    fn parse_include(&mut self) -> Node {
        self.pos += "@include".len();

        self.skip_ws();
        self.expect_char('(');

        let inner = self.read_until_unbalanced(')', '(');
        let mut parts = inner.splitn(2, ';').map(|s| s.trim());
        let path = parts.next().unwrap_or("").to_string();
        let local_ctx = parts.next().map(parse_kv_pairs).unwrap_or_default();

        // Optional block `{ ... }`
        self.skip_ws();
        let body = if self.peek_char() == Some('{') {
            self.pos += 1; // consume '{'
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
                self.pos += 1;
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
                self.pos += "@else".len();
                continue;
            }

            text_buf.push(self.chars[self.pos]);
            self.pos += 1;
        }

        if !text_buf.is_empty() && !text_buf.is_empty() {
            nodes.push(Node::Text(text_buf.clone()));
        }

        nodes
    }

    fn parse_variable(&mut self) -> Vec<String> {
        debug_assert!(self.starts_with("{{"));
        self.pos += 2;
        let start = self.pos;

        while !self.eof() {
            if self.starts_with("}}") {
                let expr = self.src[start..self.pos].trim();
                self.pos += 2;

                let trimmed = expr.trim();
                if trimmed == "content" {
                    return vec!["__CONTENT__".to_string()];
                }
                return parse_variable_path(trimmed);
            }
            self.pos += 1;
        }

        parse_variable_path(self.src[start..].trim())
    }

    fn parse_if(&mut self) -> Node {
        self.pos += "@if".len();

        self.skip_ws();
        self.expect_char('(');

        let expr = self.read_until_unbalanced(')', '(');
        let cond = parse_bool_expr(expr.trim()); // NEW

        self.skip_ws();
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        let mut conditions = Vec::new();
        conditions.push((cond, body));
        let mut otherwise: Option<Vec<Node>> = None;

        loop {
            self.skip_ws();

            if self.starts_with("@else") {
                self.pos += "@else".len();
                self.skip_ws();

                if self.starts_with("if") {
                    // '@else if (...) { ... }'
                    self.pos += "if".len();
                    self.skip_ws();
                    self.expect_char('(');
                    let expr = self.read_until_unbalanced(')', '(');
                    let cond = parse_bool_expr(expr.trim()); // NEW

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
        self.pos += "@for".len();

        // Allow optional whitespace before '('
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
        let start_pos = self.pos;
        let mut depth = 0;

        while !self.eof() {
            let c = self.chars[self.pos];

            if c == start_pair {
                depth += 1;
            } else if c == end {
                if depth == 0 {
                    let s = self.src[start_pos..self.pos].to_string();
                    self.pos += 1;
                    return s;
                } else {
                    depth -= 1;
                }
            }
            self.pos += 1;
        }

        self.src[start_pos..].to_string()
    }

    fn expect_char(&mut self, expected: char) {
        self.skip_ws();
        if self.peek_char() == Some(expected) {
            self.pos += 1;
        }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn starts_with(&self, s: &str) -> bool {
        let end = self.pos + s.len();
        end <= self.chars.len() && &self.src[self.pos..end] == s
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
    let normalized = s.split_whitespace().collect::<Vec<_>>().join(" ");

    normalized
        .split(' ')
        .filter_map(|pair| {
            let mut kv = pair.splitn(2, '=').map(|x| x.trim());
            let k = kv.next()?;
            let v = kv.next()?;

            if (v.starts_with('"') && v.ends_with('"'))
                || (v.starts_with('\'') && v.ends_with('\''))
            {
                let inner = &v[1..v.len() - 1];
                Some((
                    k.to_string(),
                    LocalValue::Literal(serde_json::Value::String(inner.to_string())),
                ))
            } else {
                Some((k.to_string(), LocalValue::Path(parse_variable_path(v))))
            }
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
enum Tok {
    Ident(String),
    And,
    Or,
    LParen,
    RParen,
}

fn tokenize_bool(s: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut cur = String::new();

    let push_cur = |cur: &mut String, toks: &mut Vec<Tok>| {
        if cur.is_empty() {
            return;
        }
        let w = cur.trim().to_string();
        cur.clear();
        match w.as_str() {
            "&&" | "and" => toks.push(Tok::And),
            "||" | "or" => toks.push(Tok::Or),
            _ => toks.push(Tok::Ident(w)),
        }
    };

    for c in s.chars() {
        match c {
            '(' => {
                push_cur(&mut cur, &mut toks);
                toks.push(Tok::LParen);
            }
            ')' => {
                push_cur(&mut cur, &mut toks);
                toks.push(Tok::RParen);
            }
            c if c.is_whitespace() => {
                push_cur(&mut cur, &mut toks);
            }
            _ => cur.push(c),
        }
    }
    push_cur(&mut cur, &mut toks);
    toks
}

struct Cursor {
    toks: Vec<Tok>,
    pos: usize,
}

impl Cursor {
    fn new(toks: Vec<Tok>) -> Self {
        Self { toks, pos: 0 }
    }
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
}

fn parse_bool_expr(s: &str) -> Condition {
    let toks = tokenize_bool(s);
    let mut cur = Cursor::new(toks);
    parse_expr(&mut cur)
}

fn parse_expr(cur: &mut Cursor) -> Condition {
    // expr := term ( "or" term )*
    let left = parse_term(cur);
    let mut parts = vec![left];

    while let Some(Tok::Or) = cur.peek() {
        cur.next(); // consume 'or'
        let rhs = parse_term(cur);
        parts.push(rhs);
    }

    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        Condition::Or(parts)
    }
}

fn parse_term(cur: &mut Cursor) -> Condition {
    // term := factor ( "and" factor )*
    let left = parse_factor(cur);
    let mut parts = vec![left];

    while let Some(Tok::And) = cur.peek() {
        cur.next(); // consume 'and'
        let rhs = parse_factor(cur);
        parts.push(rhs);
    }

    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        Condition::And(parts)
    }
}

fn parse_factor(cur: &mut Cursor) -> Condition {
    match cur.peek() {
        Some(Tok::LParen) => {
            cur.next(); // '('
            let inner = parse_expr(cur);
            match cur.next() {
                Some(Tok::RParen) => inner,
                _ => inner, // tolerate missing ')'
            }
        }
        Some(Tok::Ident(_)) => match cur.next() {
            Some(Tok::Ident(s)) => match s.as_str() {
                "true" => Condition::Literal(true),
                "false" => Condition::Literal(false),
                _ => Condition::Path(parse_variable_path(&s)),
            },
            _ => Condition::Literal(false),
        },
        _ => Condition::Literal(false),
    }
}

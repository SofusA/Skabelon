use crate::nodes::{ForLoop, If, Include, Node};

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
        let local_ctx = parts
            .next()
            .map(parse_kv_pairs_to_values)
            .unwrap_or_default();

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
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                self.pos += 1;
                break;
            }

            if self.starts_with("{{") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(Node::VariableBlock(self.parse_variable()));
                continue;
            }

            if self.starts_with("@include") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(self.parse_include());
                continue;
            }

            if self.starts_with("@if") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(self.parse_if());
                continue;
            }

            if self.starts_with("@for") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(self.parse_for());
                continue;
            }

            if self.starts_with("@else") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }

                text_buf.push_str("@else");
                self.pos += "@else".len();
                continue;
            }

            text_buf.push(self.chars[self.pos]);
            self.pos += 1;
        }

        if !text_buf.is_empty() {
            push_text_with_content_placeholders(&mut nodes, &text_buf);
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
                return parse_variable_path(expr);
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
        let path = parse_variable_path(expr.trim());

        self.skip_ws();
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        let mut conditions = Vec::new();
        conditions.push((path, body));
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
                    let path = parse_variable_path(expr.trim());

                    self.skip_ws();
                    self.expect_char('{');
                    let body = self.parse_nodes(Some('}'));

                    conditions.push((path, body));
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

fn push_text_with_content_placeholders(nodes: &mut Vec<Node>, text: &str) {
    let mut start = 0;
    for (idx, _) in text.match_indices("<content-slot>") {
        if idx > start {
            nodes.push(Node::Text(text[start..idx].to_string()));
        }
        nodes.push(Node::ContentPlaceholder);
        start = idx + "<content-slot>".len();
    }
    if start < text.len() {
        nodes.push(Node::Text(text[start..].to_string()));
    }
}

fn parse_kv_pairs_to_values(s: &str) -> Vec<(String, serde_json::Value)> {
    s.split(',')
        .filter_map(|pair| {
            let mut kv = pair.splitn(2, '=').map(|x| x.trim());
            let k = kv.next()?;
            let v = kv.next()?;
            let val = parse_literal_to_value(v);
            Some((k.to_string(), val))
        })
        .collect()
}

fn parse_literal_to_value(raw: &str) -> serde_json::Value {
    let t = raw.trim();

    if (t.starts_with('\'') && t.ends_with('\'')) || (t.starts_with('"') && t.ends_with('"')) {
        let inner = &t[1..t.len() - 1];
        serde_json::Value::String(inner.to_string())
    } else {
        match t {
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            "null" => serde_json::Value::Null,
            _ => {
                if let Ok(i) = t.parse::<i64>() {
                    serde_json::Value::Number(i.into())
                } else if let Ok(f) = t.parse::<f64>() {
                    serde_json::json!(f)
                } else {
                    serde_json::Value::String(t.to_string())
                }
            }
        }
    }
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

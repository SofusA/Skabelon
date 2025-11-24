use crate::nodes::{ForLoop, If, Include, Node};

pub(crate) fn parse_template(input: &str) -> Vec<Node> {
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

    fn parse_script(&mut self) -> Node {
        self.pos += "@script".len();
        self.expect_char('{');
        let body = self.read_until_unbalanced('}', '{');
        Node::Script(body)
    }

    fn parse_include(&mut self) -> Node {
        self.pos += "@include(".len();
        let inner = self.read_until_unbalanced(')', '(');
        let mut parts = inner.splitn(2, ';').map(|s| s.trim());
        let path = parts.next().unwrap_or("").to_string();
        let local_ctx = parts.next().map(parse_kv_pairs_raw).unwrap_or_default();

        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

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

            // NEW: @script{ ... }
            if self.starts_with("@script{") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(self.parse_script());
                continue;
            }

            if self.starts_with("@include(") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(self.parse_include());
                continue;
            }

            if self.starts_with("@if(") {
                if !text_buf.is_empty() {
                    push_text_with_content_placeholders(&mut nodes, &text_buf);
                    text_buf.clear();
                }
                nodes.push(self.parse_if());
                continue;
            }

            if self.starts_with("@for(") {
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

    fn parse_variable(&mut self) -> String {
        debug_assert!(self.starts_with("{{"));
        self.pos += 2; // consume {{
        let start = self.pos;

        while !self.eof() {
            if self.starts_with("}}") {
                let var = self.src[start..self.pos].trim().to_string();
                self.pos += 2; // consume }}
                return var;
            }
            self.pos += 1;
        }
        // Unterminated variable; return whatever we have
        self.src[start..].trim().to_string()
    }

    fn parse_if(&mut self) -> Node {
        // consume "@if("
        self.pos += "@if(".len();
        let expr = self.read_until_unbalanced(')', '(');
        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        // optional @else { ... }
        self.skip_ws();
        let mut otherwise = None;
        if self.starts_with("@else") {
            self.pos += "@else".len();
            self.skip_ws();
            self.expect_char('{');
            let else_body = self.parse_nodes(Some('}'));
            otherwise = Some(else_body);
        }

        Node::If(If {
            conditions: vec![(expr.trim().to_string(), body)],
            otherwise,
        })
    }

    fn parse_for(&mut self) -> Node {
        // consume "@for("
        self.pos += "@for(".len();
        let for_expr = self.read_until_unbalanced(')', '(');
        // Expected form: "item in items"
        let (value, container) = parse_for_expression(&for_expr);

        self.expect_char('{');
        let body = self.parse_nodes(Some('}'));

        Node::Forloop(ForLoop {
            value,
            container,
            body,
        })
    }

    fn read_until_unbalanced(&mut self, end: char, start_pair: char) -> String {
        // Read until we meet `end` char at nesting level 0.
        // For simplicity here, we assume no nested parens inside expr.
        // We still handle cases where `start_pair` might appear.
        let start_pos = self.pos;
        let mut depth = 0;

        while !self.eof() {
            let c = self.chars[self.pos];

            if c == start_pair {
                depth += 1;
            } else if c == end {
                if depth == 0 {
                    let s = self.src[start_pos..self.pos].to_string();
                    self.pos += 1; // consume end char
                    return s;
                } else {
                    depth -= 1;
                }
            }
            self.pos += 1;
        }
        // If we reach EOF, return what's left
        self.src[start_pos..].to_string()
    }

    fn expect_char(&mut self, expected: char) {
        self.skip_ws();
        if self.peek_char() == Some(expected) {
            self.pos += 1;
        } else {
            // Soft fail: treat missing delimiter as text
            // to keep parser resilient in prototype.
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
    // Parses "value in container" into (value, container)
    // Trim and split by "in" (first occurrence).
    let trimmed = expr.trim();
    let mut parts = trimmed.splitn(2, " in ");
    let value = parts.next().unwrap_or("").trim().to_string();
    let container = parts.next().unwrap_or("").trim().to_string();
    (value, container)
}

fn push_text_with_content_placeholders(nodes: &mut Vec<Node>, text: &str) {
    let mut start = 0;
    for (idx, _) in text.match_indices("@content") {
        if idx > start {
            nodes.push(Node::Text(text[start..idx].to_string()));
        }
        nodes.push(Node::ContentPlaceholder);
        start = idx + "@content".len();
    }
    if start < text.len() {
        nodes.push(Node::Text(text[start..].to_string()));
    }
}

fn parse_kv_pairs_raw(s: &str) -> Vec<(String, String)> {
    s.split(',')
        .filter_map(|pair| {
            let mut kv = pair.splitn(2, '=').map(|x| x.trim());
            let k = kv.next()?;
            let v = kv.next()?;
            Some((k.to_string(), v.to_string())) // keep raw expression
        })
        .collect()
}

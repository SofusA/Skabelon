use serde_json::{Number, Value};

use crate::nodes::{CompareOp, Condition, ForLoop, If, Include, LocalValue, Node, Operand};

pub fn parse_template(input: &str) -> Vec<Node> {
    Parser::new(input).parse_nodes(None)
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

    fn parse_nodes(&mut self, end_on: Option<char>) -> Vec<Node> {
        let mut nodes = Vec::new();
        let mut text_buf = String::new();
        let mut closed = end_on.is_none();

        while !self.eof() {
            if end_on.is_some_and(|end| self.peek_char() == Some(end)) {
                self.flush_text(&mut nodes, &mut text_buf);
                self.advance_one();
                closed = true;
                break;
            }

            if self.starts_with("{{") {
                self.flush_text(&mut nodes, &mut text_buf);
                nodes.push(self.parse_variable_node());
                continue;
            }

            if self.starts_control("@defer") {
                self.flush_text(&mut nodes, &mut text_buf);
                nodes.push(self.parse_defer());
                continue;
            }

            if self.starts_control("@if") {
                self.flush_text(&mut nodes, &mut text_buf);
                nodes.push(self.parse_if());
                continue;
            }

            if self.starts_control("@for") {
                self.flush_text(&mut nodes, &mut text_buf);
                nodes.push(self.parse_for());
                continue;
            }

            if self.starts_control("@else") {
                self.flush_text(&mut nodes, &mut text_buf);
                text_buf.push_str("@else");
                self.byte_offset += "@else".len();
                continue;
            }

            let start = self.byte_offset;
            self.advance_until_special(end_on);

            if self.byte_offset == start {
                if let Some(ch) = self.peek_char() {
                    text_buf.push(ch);
                    self.advance_one();
                } else {
                    break;
                }
            } else {
                text_buf.push_str(&self.src[start..self.byte_offset]);
            }
        }

        self.flush_text(&mut nodes, &mut text_buf);

        if !closed {
            nodes.push(self.error_node(format!("Expected closing '{}'", end_on.unwrap())));
        }

        nodes
    }

    fn parse_defer(&mut self) -> Node {
        self.byte_offset += "@defer".len();
        self.skip_whitespace();

        if !self.expect_char('(') {
            return self.error_node("Expected '(' after @defer");
        }

        let (inner, closed) = self.read_until_unbalanced(')', '(');

        if !closed {
            return self.error_node("Expected closing ')' for @defer");
        }

        let split_at = find_top_level_char(&inner, ';');

        let path = split_at
            .map(|i| inner[..i].trim().to_owned())
            .unwrap_or_else(|| inner.trim().to_owned());

        if path.is_empty() {
            return self.error_node("Expected template path inside @defer");
        }

        let local_ctx = split_at
            .map(|i| parse_kv_pairs(&inner[i + 1..]))
            .unwrap_or_default();

        self.skip_whitespace();

        let body = if self.peek_char() == Some('{') {
            self.advance_one();
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

    fn parse_variable_node(&mut self) -> Node {
        self.byte_offset += 2;
        let start = self.byte_offset;

        while !self.eof() {
            if self.starts_with("}}") {
                let expr = self.src[start..self.byte_offset].trim();
                self.byte_offset += 2;

                return if expr == "content" {
                    Node::VariableBlock(vec!["__CONTENT__".to_owned()])
                } else if expr.is_empty() {
                    self.error_node("Expected expression inside variable block")
                } else {
                    Node::VariableBlock(parse_variable_path(expr))
                };
            }

            self.advance_one();
        }

        Node::Error(format!(
            "Expected closing brackets for variable block at byte {}",
            start
        ))
    }

    fn parse_if(&mut self) -> Node {
        self.byte_offset += "@if".len();
        self.skip_whitespace();

        if !self.expect_char('(') {
            return self.error_node("Expected '(' after @if");
        }

        let (expr, closed) = self.read_until_unbalanced(')', '(');

        if !closed {
            return self.error_node("Expected closing ')' for @if condition");
        }

        let cond = parse_bool_expr(expr.trim());

        self.skip_whitespace();

        if !self.expect_char('{') {
            return self.error_node("Expected '{' after @if condition");
        }

        let body = self.parse_nodes(Some('}'));
        let mut conditions = vec![(cond, body)];
        let mut otherwise = None;

        loop {
            let save = self.byte_offset;
            self.skip_whitespace();

            if !self.starts_control("@else") {
                self.byte_offset = save;
                break;
            }

            self.byte_offset += "@else".len();
            self.skip_whitespace();

            if self.starts_keyword("if") {
                self.byte_offset += "if".len();
                self.skip_whitespace();

                if !self.expect_char('(') {
                    otherwise = Some(vec![self.error_node("Expected '(' after @else if")]);
                    break;
                }

                let (expr, closed) = self.read_until_unbalanced(')', '(');

                if !closed {
                    otherwise = Some(vec![
                        self.error_node("Expected closing ')' for @else if condition"),
                    ]);
                    break;
                }

                let cond = parse_bool_expr(expr.trim());

                self.skip_whitespace();

                if !self.expect_char('{') {
                    otherwise = Some(vec![
                        self.error_node("Expected '{' after @else if condition"),
                    ]);
                    break;
                }

                let body = self.parse_nodes(Some('}'));
                conditions.push((cond, body));
                continue;
            }

            self.skip_whitespace();

            if !self.expect_char('{') {
                otherwise = Some(vec![self.error_node("Expected '{' after @else")]);
                break;
            }

            otherwise = Some(self.parse_nodes(Some('}')));
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

        if !self.expect_char('(') {
            return self.error_node("Expected '(' after @for");
        }

        let (for_expr, closed) = self.read_until_unbalanced(')', '(');

        if !closed {
            return self.error_node("Expected closing ')' for @for expression");
        }

        let (value, container_str) = parse_for_expression(&for_expr);

        if value.trim().is_empty() || container_str.trim().is_empty() {
            return self.error_node("Expected @for expression like '@for (item in items)'");
        }

        let container = parse_variable_path(container_str.trim());

        self.skip_whitespace();

        if !self.expect_char('{') {
            return self.error_node("Expected '{' after @for expression");
        }

        let body = self.parse_nodes(Some('}'));

        Node::Forloop(ForLoop {
            value,
            container,
            body,
        })
    }

    fn read_until_unbalanced(&mut self, end: char, start_pair: char) -> (String, bool) {
        let start_position = self.byte_offset;
        let mut depth = 0usize;
        let mut quote = None;
        let mut escaped = false;

        for (i, c) in self.src[self.byte_offset..].char_indices() {
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
                continue;
            }

            match c {
                '"' | '\'' => quote = Some(c),
                c if c == start_pair => depth += 1,
                c if c == end => {
                    if depth == 0 {
                        let end_byte = self.byte_offset + i;
                        let s = self.src[start_position..end_byte].to_owned();
                        self.byte_offset = end_byte + end.len_utf8();
                        return (s, true);
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }

        let s = self.src[start_position..].to_owned();
        self.byte_offset = self.src.len();
        (s, false)
    }

    #[inline]
    fn error_node(&self, message: impl Into<String>) -> Node {
        Node::Error(format!("{} at byte {}", message.into(), self.byte_offset))
    }

    #[inline]
    fn flush_text(&self, nodes: &mut Vec<Node>, text_buf: &mut String) {
        if !text_buf.is_empty() {
            nodes.push(Node::Text(std::mem::take(text_buf)));
        }
    }

    #[inline]
    fn advance_until_special(&mut self, end_on: Option<char>) {
        let bytes = self.src.as_bytes();

        while self.byte_offset < bytes.len() {
            let b = bytes[self.byte_offset];

            if b == b'@' || b == b'{' || end_on == Some('}') && b == b'}' {
                break;
            }

            self.byte_offset += 1;
        }
    }

    #[inline]
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if !c.is_whitespace() {
                break;
            }

            self.byte_offset += c.len_utf8();
        }
    }

    #[inline]
    fn expect_char(&mut self, expected: char) -> bool {
        self.skip_whitespace();

        if self.peek_char() == Some(expected) {
            self.byte_offset += expected.len_utf8();
            true
        } else {
            false
        }
    }

    #[inline]
    fn peek_char(&self) -> Option<char> {
        self.src.get(self.byte_offset..)?.chars().next()
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

    #[inline]
    fn starts_control(&self, s: &str) -> bool {
        self.starts_with(s) && self.boundary_after(s.len())
    }

    #[inline]
    fn starts_keyword(&self, s: &str) -> bool {
        self.starts_with(s) && self.boundary_after(s.len())
    }

    #[inline]
    fn boundary_after(&self, len: usize) -> bool {
        let i = self.byte_offset + len;

        self.src
            .get(i..)
            .and_then(|s| s.chars().next())
            .map(|c| c.is_whitespace() || c == '(' || c == '{')
            .unwrap_or(true)
    }
}

fn parse_for_expression(expr: &str) -> (String, String) {
    let trimmed = expr.trim();

    if let Some(i) = find_top_level_keyword(trimmed, "in") {
        return (
            trimmed[..i].trim().to_owned(),
            trimmed[i + "in".len()..].trim().to_owned(),
        );
    }

    (trimmed.to_owned(), String::new())
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

        pairs.push((key.to_owned(), parse_local_value(value.trim())));
    }

    pairs
}

fn parse_local_value(v: &str) -> LocalValue {
    if let Some(s) = unquote(v) {
        return LocalValue::Literal(Value::String(s));
    }

    match v {
        "true" => LocalValue::Literal(Value::Bool(true)),
        "false" => LocalValue::Literal(Value::Bool(false)),
        "null" => LocalValue::Literal(Value::Null),
        _ => {
            if let Ok(i) = v.parse::<i64>() {
                LocalValue::Literal(Value::Number(i.into()))
            } else if let Ok(f) = v.parse::<f64>() {
                Number::from_f64(f)
                    .map(|n| LocalValue::Literal(Value::Number(n)))
                    .unwrap_or_else(|| LocalValue::Path(parse_variable_path(v)))
            } else {
                LocalValue::Path(parse_variable_path(v))
            }
        }
    }
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
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                break;
            }
        }

        return (&s[start..i], i);
    }

    let mut quote = None;
    let mut escaped = false;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;

    while i < s.len() {
        let ch = char_at(s, i).unwrap();

        if let Some(q) = quote {
            i += ch.len_utf8();

            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
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
    let mut quote = None;
    let mut escaped = false;

    for c in expr.trim().chars() {
        if let Some(q) = quote {
            if escaped {
                current.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            } else {
                current.push(c);
            }

            continue;
        }

        match c {
            '"' | '\'' if in_brackets => quote = Some(c),
            '[' => {
                push_trimmed(&mut parts, &mut current);
                in_brackets = true;
            }
            ']' if in_brackets => {
                push_trimmed(&mut parts, &mut current);
                in_brackets = false;
            }
            '.' if !in_brackets => push_trimmed(&mut parts, &mut current),
            c if c.is_whitespace() && !in_brackets => {}
            _ => current.push(c),
        }
    }

    push_trimmed(&mut parts, &mut current);
    parts
}

fn push_trimmed(parts: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();

    if !trimmed.is_empty() {
        parts.push(trimmed.to_owned());
    }

    current.clear();
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
    let mut cur = Cursor::new(tokenize_bool(s));
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
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == quote {
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
            '=' if chars.peek() == Some(&'=') => {
                push_ident_token(&mut cur, &mut tokens);
                chars.next();
                tokens.push(Token::Eq);
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
            '&' if chars.peek() == Some(&'&') => {
                push_ident_token(&mut cur, &mut tokens);
                chars.next();
                tokens.push(Token::And);
            }
            '|' if chars.peek() == Some(&'|') => {
                push_ident_token(&mut cur, &mut tokens);
                chars.next();
                tokens.push(Token::Or);
            }
            c if c.is_whitespace() => push_ident_token(&mut cur, &mut tokens),
            _ => cur.push(c),
        }
    }

    push_ident_token(&mut cur, &mut tokens);
    tokens
}

fn push_ident_token(cur: &mut String, tokens: &mut Vec<Token>) {
    let w = cur.trim();

    if !w.is_empty() {
        match w {
            "and" => tokens.push(Token::And),
            "or" => tokens.push(Token::Or),
            "not" => tokens.push(Token::Not),
            _ => tokens.push(Token::Ident(w.to_owned())),
        }
    }

    cur.clear();
}

fn parse_expr(cur: &mut Cursor) -> Condition {
    let left = parse_term(cur);
    let mut parts = vec![left];

    while matches!(cur.peek(), Some(Token::Or)) {
        cur.next();
        parts.push(parse_term(cur));
    }

    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        Condition::Or(parts)
    }
}

fn parse_term(cur: &mut Cursor) -> Condition {
    let left = parse_unary(cur);
    let mut parts = vec![left];

    while matches!(cur.peek(), Some(Token::And)) {
        cur.next();
        parts.push(parse_unary(cur));
    }

    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        Condition::And(parts)
    }
}

fn parse_unary(cur: &mut Cursor) -> Condition {
    if matches!(cur.peek(), Some(Token::Not)) {
        cur.next();
        Condition::Not(Box::new(parse_unary(cur)))
    } else {
        parse_factor(cur)
    }
}

fn parse_factor(cur: &mut Cursor) -> Condition {
    match cur.peek() {
        Some(Token::LParen) => {
            cur.next();
            let inner = parse_expr(cur);

            if matches!(cur.peek(), Some(Token::RParen)) {
                cur.next();
            }

            inner
        }
        Some(Token::Ident(_)) => {
            let left_ident = match cur.next() {
                Some(Token::Ident(s)) => s,
                _ => String::new(),
            };

            if let Some(op) = cur.peek().and_then(parse_compare_op) {
                cur.next();

                return Condition::Compare {
                    left: operand_from_expr(&left_ident),
                    op,
                    right: parse_operand(cur.next()),
                };
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
        Operand::Literal(parse_literal(Some(Token::Ident(s.to_owned()))))
    } else {
        Operand::Path(parse_variable_path(s))
    }
}

fn is_literal_expr(s: &str) -> bool {
    matches!(s, "true" | "false" | "null")
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
        Some(Token::Ident(s)) => match s.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "null" => Value::Null,
            _ => {
                if let Ok(i) = s.parse::<i64>() {
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
        },
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
    let mut out = String::with_capacity(inner.len());
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
    find_top_level(s, |_, ch| ch == target)
}

fn find_top_level_keyword(s: &str, keyword: &str) -> Option<usize> {
    find_top_level(s, |i, _| {
        s[i..].starts_with(keyword) && is_keyword_boundary(s, i, keyword.len())
    })
}

fn find_top_level<F>(s: &str, mut pred: F) -> Option<usize>
where
    F: FnMut(usize, char) -> bool,
{
    let mut quote = None;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for (i, ch) in s.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
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
            _ if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && pred(i, ch) => {
                return Some(i);
            }
            _ => {}
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

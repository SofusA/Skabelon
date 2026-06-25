use crate::{
    nodes::{CompareOp, Condition, ForLoop, If, Include, LocalValue, Node, Operand},
    templates::Templates,
};
use serde_json::Value;
use std::collections::HashMap;

pub struct ContextStack<'a> {
    scopes: Vec<HashMap<String, Value>>,
    global: &'a Value,
}

impl<'a> ContextStack<'a> {
    pub fn new(global: &'a Value) -> Self {
        Self {
            scopes: Vec::new(),
            global,
        }
    }

    pub fn push_scope_with_capacity(&mut self, capacity: usize) {
        self.scopes.push(HashMap::with_capacity(capacity));
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn set(&mut self, key: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key, value);
        }
    }

    pub fn set_str(&mut self, key: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key.to_owned(), value);
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(key) {
                return Some(val);
            }
        }

        self.global.get(key)
    }
}

fn trim_single_trailing_space(out: &mut String) -> bool {
    if out.ends_with(' ') {
        out.pop();
        true
    } else {
        false
    }
}

fn estimate_capacity(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Text(s) => s.len(),
            Node::If(If {
                conditions,
                otherwise,
            }) => {
                let condition_max = conditions
                    .iter()
                    .map(|(_, body)| estimate_capacity(body))
                    .max()
                    .unwrap_or(0);

                let otherwise_len = otherwise
                    .as_ref()
                    .map(|body| estimate_capacity(body))
                    .unwrap_or(0);

                condition_max.max(otherwise_len)
            }
            Node::Forloop(ForLoop { body, .. }) => estimate_capacity(body),
            Node::Include(Include { body, .. }) => estimate_capacity(body),
            _ => 0,
        })
        .sum()
}

pub fn render_nodes(
    nodes: &[Node],
    ctx_stack: &mut ContextStack,
    templates: &Templates,
    content_html: Option<&str>,
) -> String {
    let mut out = String::with_capacity(estimate_capacity(nodes));
    render_nodes_into(nodes, ctx_stack, templates, content_html, &mut out);
    out
}

fn render_nodes_into(
    nodes: &[Node],
    ctx_stack: &mut ContextStack,
    templates: &Templates,
    content_html: Option<&str>,
    out: &mut String,
) {
    let mut suppress_one_leading_space = false;

    for node in nodes {
        match node {
            Node::Text(s) => {
                if suppress_one_leading_space {
                    if let Some(rest) = s.strip_prefix(' ') {
                        out.push_str(rest);
                    } else {
                        out.push_str(s);
                    }
                    suppress_one_leading_space = false;
                } else {
                    out.push_str(s);
                }
            }

            Node::VariableBlock(path) => {
                if is_content_path(path) {
                    write_content(content_html, out, &mut suppress_one_leading_space);
                } else if let Some(val) = resolve_path(path, ctx_stack) {
                    value_to_string_into(val, out);
                    suppress_one_leading_space = false;
                }
            }

            Node::If(If {
                conditions,
                otherwise,
            }) => {
                let before = out.len();
                let mut taken = false;

                for (cond, body) in conditions {
                    if evaluate_condition(cond, ctx_stack) {
                        render_nodes_into(body, ctx_stack, templates, content_html, out);
                        taken = true;
                        break;
                    }
                }

                if !taken && let Some(body) = otherwise {
                    render_nodes_into(body, ctx_stack, templates, content_html, out);
                }

                if out.len() == before {
                    let popped = trim_single_trailing_space(out);
                    suppress_one_leading_space = !popped;
                } else {
                    suppress_one_leading_space = false;
                }
            }

            Node::Forloop(ForLoop {
                value,
                container,
                body,
            }) => {
                let before = out.len();

                let items = resolve_path(container, ctx_stack)
                    .and_then(Value::as_array)
                    .cloned();

                if let Some(items) = items {
                    ctx_stack.push_scope_with_capacity(2);

                    for (i, item) in items.into_iter().enumerate() {
                        ctx_stack.set(value.clone(), item);
                        ctx_stack.set_str("index", Value::from(i));
                        render_nodes_into(body, ctx_stack, templates, content_html, out);
                    }

                    ctx_stack.pop_scope();
                }

                if out.len() == before {
                    let popped = trim_single_trailing_space(out);
                    suppress_one_leading_space = !popped;
                } else {
                    suppress_one_leading_space = false;
                }
            }

            Node::Include(Include {
                path,
                body,
                local_ctx,
            }) => {
                if let Some(partial_nodes) = templates.get(path) {
                    let parent_rendered_content = render_nodes(body, ctx_stack, templates, None);

                    let null_global = Value::Null;
                    let mut partial_stack = ContextStack::new(&null_global);
                    partial_stack.push_scope_with_capacity(local_ctx.len());

                    for (k, local_val) in local_ctx {
                        let value = match local_val {
                            LocalValue::Literal(val) => val.clone(),
                            LocalValue::Path(path) => resolve_path(path, ctx_stack)
                                .cloned()
                                .unwrap_or(Value::Null),
                        };

                        partial_stack.set(k.clone(), value);
                    }

                    let before = out.len();

                    render_nodes_into(
                        partial_nodes,
                        &mut partial_stack,
                        templates,
                        Some(&parent_rendered_content),
                        out,
                    );

                    partial_stack.pop_scope();

                    if out.len() == before {
                        let popped = trim_single_trailing_space(out);
                        suppress_one_leading_space = !popped;
                    } else {
                        suppress_one_leading_space = false;
                    }
                } else {
                    out.push_str("<!-- Missing defer: ");
                    out.push_str(path);
                    out.push_str(" -->");
                    suppress_one_leading_space = false;
                }
            }

            Node::ContentPlaceholder => {
                write_content(content_html, out, &mut suppress_one_leading_space);
            }
            Node::Error(msg) => {
                out.push_str("<template-error>");
                out.push_str(msg);
                out.push_str("</template-error>");
            }
        }
    }
}

fn is_content_path(path: &[String]) -> bool {
    path.len() == 1 && path[0] == "__CONTENT__"
}

fn write_content(
    content_html: Option<&str>,
    out: &mut String,
    suppress_one_leading_space: &mut bool,
) {
    match content_html {
        Some(html) if !html.is_empty() => {
            out.push_str(html);
            *suppress_one_leading_space = false;
        }
        _ => {
            let popped = trim_single_trailing_space(out);
            *suppress_one_leading_space = !popped;
        }
    }
}

fn evaluate_condition(cond: &Condition, ctx_stack: &ContextStack) -> bool {
    match cond {
        Condition::Literal(b) => *b,
        Condition::Path(path) => evaluate_path_truthiness(path, ctx_stack),
        Condition::And(conds) => conds.iter().all(|c| evaluate_condition(c, ctx_stack)),
        Condition::Or(conds) => conds.iter().any(|c| evaluate_condition(c, ctx_stack)),
        Condition::Not(inner) => !evaluate_condition(inner, ctx_stack),
        Condition::Compare { left, op, right } => {
            let l = resolve_operand(left, ctx_stack);
            let r = resolve_operand(right, ctx_stack);

            match (l, r) {
                (Some(lv), Some(rv)) => compare_values(lv, op, rv),
                _ => false,
            }
        }
    }
}

fn resolve_operand<'a>(opnd: &'a Operand, ctx_stack: &'a ContextStack) -> Option<&'a Value> {
    match opnd {
        Operand::Literal(v) => Some(v),
        Operand::Path(p) => resolve_path(p, ctx_stack),
    }
}

fn compare_values(left: &Value, op: &CompareOp, right: &Value) -> bool {
    match (left, right) {
        (Value::String(ls), Value::String(rs)) => match op {
            CompareOp::Eq => ls == rs,
            CompareOp::Ne => ls != rs,
            CompareOp::Lt => ls < rs,
            CompareOp::Gt => ls > rs,
            CompareOp::Le => ls <= rs,
            CompareOp::Ge => ls >= rs,
        },
        (Value::Number(ln), Value::Number(rn)) => match (ln.as_f64(), rn.as_f64()) {
            (Some(lf), Some(rf)) => match op {
                CompareOp::Eq => lf == rf,
                CompareOp::Ne => lf != rf,
                CompareOp::Lt => lf < rf,
                CompareOp::Gt => lf > rf,
                CompareOp::Le => lf <= rf,
                CompareOp::Ge => lf >= rf,
            },
            _ => match op {
                CompareOp::Eq => ln == rn,
                CompareOp::Ne => ln != rn,
                _ => false,
            },
        },
        (Value::Bool(lb), Value::Bool(rb)) => match op {
            CompareOp::Eq => lb == rb,
            CompareOp::Ne => lb != rb,
            _ => false,
        },
        _ => match op {
            CompareOp::Eq => left == right,
            CompareOp::Ne => left != right,
            _ => false,
        },
    }
}

fn evaluate_path_truthiness(path: &[String], ctx_stack: &ContextStack) -> bool {
    if path.len() == 1 {
        match path[0].as_str() {
            "true" => return true,
            "false" => return false,
            raw => {
                if let Ok(num) = raw.parse::<f64>() {
                    return num != 0.0;
                }
            }
        }
    }

    match resolve_path(path, ctx_stack) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Null) => false,
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        None => false,
    }
}

fn value_to_string_into(v: &Value, out: &mut String) {
    match v {
        Value::String(s) => out.push_str(s),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::Null => {}
        other => out.push_str(&other.to_string()),
    }
}

fn resolve_path<'a>(path: &[String], ctx_stack: &'a ContextStack) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let mut value = ctx_stack.get(first)?;

    for key in rest {
        value = match value {
            Value::Object(map) => map.get(key)?,
            Value::Array(arr) => arr.get(key.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }

    Some(value)
}

use crate::{
    nodes::{CompareOp, Condition, ForLoop, If, Include, LocalValue, Node, Operand},
    templates::Templates,
};
use serde_json::Value;
use std::collections::HashMap;

pub struct ContextStack<'a> {
    scopes: Vec<HashMap<String, serde_json::Value>>,
    global: &'a serde_json::Value,
}

impl<'a> ContextStack<'a> {
    pub fn new(global: &'a serde_json::Value) -> Self {
        Self {
            scopes: Vec::new(),
            global,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn set(&mut self, key: String, value: serde_json::Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key, value);
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(key) {
                return Some(val);
            }
        }
        self.global.get(key)
    }
}

pub fn render_nodes(
    nodes: &[Node],
    ctx_stack: &mut ContextStack,
    templates: &Templates,
    content_html: Option<&str>,
) -> String {
    let mut out = String::new();

    for n in nodes {
        match n {
            Node::Text(s) => out.push_str(s),

            Node::VariableBlock(path) => {
                if path.len() == 1 && path[0] == "__CONTENT__" {
                    if let Some(html) = content_html {
                        out.push_str(html);
                    }
                } else if let Some(val) = resolve_path(path, ctx_stack) {
                    out.push_str(&value_to_string(val));
                }
            }

            Node::If(If {
                conditions,
                otherwise,
            }) => {
                let mut rendered = false;
                for (cond, body) in conditions {
                    if evaluate_condition(cond, ctx_stack) {
                        out.push_str(&render_nodes(body, ctx_stack, templates, content_html));
                        rendered = true;
                        break;
                    }
                }
                if !rendered && let Some(body) = otherwise {
                    out.push_str(&render_nodes(body, ctx_stack, templates, content_html));
                }
            }

            Node::Forloop(ForLoop {
                value,
                container,
                body,
            }) => {
                let items_opt = resolve_path(container, ctx_stack)
                    .and_then(|v| v.as_array())
                    .map(|a| a.to_vec());

                if let Some(items) = items_opt {
                    ctx_stack.push_scope();
                    for item in items.into_iter().enumerate() {
                        ctx_stack.set(value.clone(), item.1);
                        ctx_stack.set("index".into(), Value::from(item.0));
                        out.push_str(&render_nodes(body, ctx_stack, templates, content_html));
                    }
                    ctx_stack.pop_scope();
                }
            }

            Node::Include(Include {
                path,
                body,
                local_ctx,
            }) => {
                if let Some(partial_nodes) = templates.get(path) {
                    let parent_rendered_content = render_nodes(body, ctx_stack, templates, None);

                    let mut partial_stack = ContextStack::new(Default::default());
                    partial_stack.push_scope();

                    for (k, local_val) in local_ctx {
                        match local_val {
                            LocalValue::Literal(val) => partial_stack.set(k.clone(), val.clone()),
                            LocalValue::Path(path) => {
                                if let Some(val) = resolve_path(path, ctx_stack) {
                                    partial_stack.set(k.clone(), val.clone());
                                } else {
                                    partial_stack.set(k.clone(), serde_json::Value::Null);
                                }
                            }
                        }
                    }

                    let rendered = render_nodes(
                        partial_nodes,
                        &mut partial_stack,
                        templates,
                        Some(&parent_rendered_content),
                    );
                    out.push_str(&rendered);

                    partial_stack.pop_scope();
                } else {
                    out.push_str(&format!("<!-- Missing defer: {} -->", path));
                }
            }

            Node::ContentPlaceholder => {
                if let Some(html) = content_html {
                    out.push_str(html);
                }
            }
        }
    }

    out
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
                (Some(lv), Some(rv)) => compare_values(&lv, op, &rv),
                _ => false,
            }
        }
    }
}

fn resolve_operand(opnd: &Operand, ctx_stack: &ContextStack) -> Option<Value> {
    match opnd {
        Operand::Literal(v) => Some(v.clone()),
        Operand::Path(p) => resolve_path(p, ctx_stack).cloned(),
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
        (Value::Number(ln), Value::Number(rn)) => {
            let lf = ln.as_f64().unwrap_or(0.0);
            let rf = rn.as_f64().unwrap_or(0.0);
            match op {
                CompareOp::Eq => lf == rf,
                CompareOp::Ne => lf != rf,
                CompareOp::Lt => lf < rf,
                CompareOp::Gt => lf > rf,
                CompareOp::Le => lf <= rf,
                CompareOp::Ge => lf >= rf,
            }
        }
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
        let raw = &path[0];
        match raw.as_str() {
            "true" => return true,
            "false" => return false,
            _ => {
                if let Ok(num) = raw.parse::<f64>() {
                    return num != 0.0;
                }
            }
        }
    }

    if let Some(val) = resolve_path(path, ctx_stack) {
        match val {
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
        }
    } else {
        false
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Null => "".into(),
        other => other.to_string(),
    }
}

fn resolve_path<'a>(path: &'a [String], ctx_stack: &'a ContextStack) -> Option<&'a Value> {
    if path.is_empty() {
        return None;
    }
    let mut value = ctx_stack.get(&path[0])?;

    for key in &path[1..] {
        match value {
            Value::Object(map) => {
                value = map.get(key)?;
            }
            Value::Array(arr) => {
                if let Ok(index) = key.parse::<usize>() {
                    value = arr.get(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(value)
}

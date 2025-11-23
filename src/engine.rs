use crate::{nodes::Node, templates::Templates};
use serde_json::Value;
use std::collections::HashMap;

pub struct ContextStack<'a> {
    scopes: Vec<HashMap<String, serde_json::Value>>,
    global: &'a HashMap<String, serde_json::Value>,
}

impl<'a> ContextStack<'a> {
    pub(crate) fn new(global: &'a HashMap<String, serde_json::Value>) -> Self {
        Self {
            scopes: Vec::new(),
            global,
        }
    }

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn set(&mut self, key: String, value: serde_json::Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key, value);
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&serde_json::Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(key) {
                return Some(val);
            }
        }
        self.global.get(key)
    }
}

pub(crate) fn render_nodes(
    nodes: &[Node],
    ctx_stack: &mut ContextStack,
    templates: &Templates,
    content_body: Option<&[Node]>,
) -> String {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(s) => out.push_str(s),
            Node::VariableBlock(var) => {
                if let Some(val) = ctx_stack.get(var) {
                    out.push_str(&value_to_string(val));
                }
            }
            Node::If(if_block) => {
                let mut rendered = false;
                for (expr, body) in &if_block.conditions {
                    if evaluate_condition(expr, ctx_stack) {
                        out.push_str(&render_nodes(body, ctx_stack, templates, content_body));
                        rendered = true;
                        break;
                    }
                }
                if !rendered && let Some(body) = &if_block.otherwise {
                    out.push_str(&render_nodes(body, ctx_stack, templates, content_body));
                }
            }
            Node::Forloop(forloop) => {
                // Acquire owned items in a short-lived scope, so the immutable borrow ends
                let items: Option<Vec<serde_json::Value>> = {
                    if let Some(serde_json::Value::Array(arr_ref)) =
                        ctx_stack.get(&forloop.container)
                    {
                        Some(arr_ref.clone()) // clone the Vec<Value>
                    } else {
                        None
                    }
                };

                if let Some(arr) = items {
                    ctx_stack.push_scope();
                    for item in arr {
                        ctx_stack.set(forloop.value.clone(), item);
                        out.push_str(&render_nodes(
                            &forloop.body,
                            ctx_stack,
                            templates,
                            content_body,
                        ));
                    }
                    ctx_stack.pop_scope();
                }
            }
            Node::Include(include) => {
                if let Some(partial_nodes) = templates.get(&include.path) {
                    out.push_str(&render_nodes(
                        partial_nodes,
                        ctx_stack,
                        templates,
                        Some(&include.body),
                    ));
                } else {
                    out.push_str(&format!("<!-- Missing include: {} -->", include.path));
                }
            }
            Node::ContentPlaceholder => {
                if let Some(content_nodes) = content_body {
                    out.push_str(&render_nodes(content_nodes, ctx_stack, templates, None));
                }
            }
        }
    }
    out
}

fn evaluate_condition(expr: &str, ctx_stack: &ContextStack) -> bool {
    let e = expr.trim();
    match e {
        "true" => true,
        "false" => false,
        _ => {
            if let Some(v) = ctx_stack.get(e) {
                match v {
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            i != 0
                        } else if let Some(u) = n.as_u64() {
                            u != 0
                        } else if let Some(f) = n.as_f64() {
                            f != 0.0
                        } else {
                            false
                        }
                    }
                    serde_json::Value::String(s) => !s.is_empty(),
                    serde_json::Value::Null => false,
                    serde_json::Value::Array(a) => !a.is_empty(),
                    serde_json::Value::Object(o) => !o.is_empty(),
                }
            } else {
                false
            }
        }
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(), // ✅ no quotes
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Null => "".into(),
        other => other.to_string(), // arrays/objects fallback
    }
}

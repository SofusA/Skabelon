use crate::{
    nodes::{ForLoop, If, Include, Node},
    templates::Templates,
};
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

    pub fn get_array_owned(&self, key: &str) -> Option<Vec<serde_json::Value>> {
        match self.get(key) {
            Some(serde_json::Value::Array(a)) => Some(a.clone()),
            _ => None,
        }
    }
}

pub(crate) fn render_nodes(
    nodes: &[Node],
    ctx_stack: &mut ContextStack,
    templates: &Templates,
    content_html: Option<&str>, // NEW: pre-rendered @content HTML
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

            Node::If(If {
                conditions,
                otherwise,
            }) => {
                let mut rendered = false;
                for (expr, body) in conditions {
                    if evaluate_condition(expr, ctx_stack) {
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
                if let Some(arr) = ctx_stack.get_array_owned(container) {
                    ctx_stack.push_scope();
                    for item in arr {
                        ctx_stack.set(value.clone(), item);
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
                    // 1) Pre-render @content with the *parent* context
                    let parent_rendered_content = render_nodes(body, ctx_stack, templates, None);

                    // 2) Build an *isolated* context for the partial (no parent access)
                    let empty_global: HashMap<String, Value> = HashMap::new();
                    let mut partial_stack = ContextStack::new(&empty_global);
                    partial_stack.push_scope();
                    for (k, v) in local_ctx {
                        partial_stack.set(k.clone(), v.clone());
                    }

                    // 3) Render partial; placeholders resolve to the pre-rendered string
                    let rendered = render_nodes(
                        partial_nodes,
                        &mut partial_stack,
                        templates,
                        Some(&parent_rendered_content),
                    );
                    out.push_str(&rendered);

                    partial_stack.pop_scope();
                } else {
                    out.push_str(&format!("<!-- Missing include: {} -->", path));
                }
            }

            Node::ContentPlaceholder => {
                if let Some(html) = content_html {
                    out.push_str(html);
                }
                // If no content provided, render nothing
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
                    Value::Bool(b) => *b,
                    Value::Number(n) => {
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
                    Value::String(s) => !s.is_empty(),
                    Value::Null => false,
                    Value::Array(a) => !a.is_empty(),
                    Value::Object(o) => !o.is_empty(),
                }
            } else {
                false
            }
        }
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
        other => other.to_string(), // arrays/objects fallback
    }
}

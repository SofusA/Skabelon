use crate::{
    nodes::{ForLoop, Include, Node},
    templates::Templates,
};
use serde_json::Value;
use std::collections::HashMap;

pub(crate) fn render_nodes(
    nodes: &[Node],
    ctx: &HashMap<String, serde_json::Value>,
    templates: &Templates,
    content_body: Option<&[Node]>,
) -> String {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(s) => out.push_str(s),
            Node::VariableBlock(var) => {
                if let Some(val) = ctx.get(var) {
                    out.push_str(&value_to_string(val));
                }
            }
            Node::If(if_block) => {
                let mut rendered = false;
                for (expr, body) in &if_block.conditions {
                    if evaluate_condition(expr, ctx) {
                        out.push_str(&render_nodes(body, ctx, templates, content_body));
                        rendered = true;
                        break;
                    }
                }
                if !rendered && let Some(body) = &if_block.otherwise {
                    out.push_str(&render_nodes(body, ctx, templates, content_body));
                }
            }
            Node::Forloop(ForLoop {
                value,
                container,
                body,
            }) => {
                if let Some(serde_json::Value::Array(arr)) = ctx.get(container) {
                    for item in arr {
                        let mut child_ctx = ctx.clone();
                        child_ctx.insert(value.clone(), item.clone());
                        out.push_str(&render_nodes(body, &child_ctx, templates, content_body));
                    }
                }
            }
            Node::Include(Include { path, body }) => {
                if let Some(partial_nodes) = templates.get(path) {
                    // Render the partial, providing its content body AST
                    out.push_str(&render_nodes(partial_nodes, ctx, templates, Some(body)));
                } else {
                    out.push_str(&format!("<!-- Missing include: {} -->", path));
                }
            }

            Node::ContentPlaceholder => {
                if let Some(content_nodes) = content_body {
                    out.push_str(&render_nodes(content_nodes, ctx, templates, None));
                }
            }
        }
    }
    out
}

fn evaluate_condition(expr: &str, ctx: &HashMap<String, Value>) -> bool {
    let e = expr.trim();
    match e {
        "true" => true,
        "false" => false,
        _ => {
            if let Some(v) = ctx.get(e) {
                match v {
                    Value::Bool(b) => *b,
                    Value::Number(n) => {
                        // Non-zero considered truthy
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

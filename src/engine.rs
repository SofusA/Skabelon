use crate::{nodes::Node, templates::Templates};
use rhai::{Array, Dynamic, Engine, Scope};
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
}

pub(crate) fn render_nodes(
    nodes: &[Node],
    ctx_stack: &mut ContextStack,
    templates: &Templates,
    content_html: Option<&str>, // NEW: pre-rendered @content HTML
) -> String {
    let mut out = String::new();
    let engine = Engine::new();

    for n in nodes {
        match n {
            Node::Text(s) => out.push_str(s),

            Node::VariableBlock(expr) => {
                let mut scope = create_scope(ctx_stack);
                if let Ok(val) = engine.eval_with_scope::<Dynamic>(&mut scope, expr) {
                    out.push_str(&val.to_string());
                }
            }

            Node::If(if_block) => {
                let mut rendered = false;
                for (expr, body) in &if_block.conditions {
                    let mut scope = create_scope(ctx_stack);
                    if let Ok(cond) = engine.eval_with_scope::<bool>(&mut scope, expr)
                        && cond
                    {
                        out.push_str(&render_nodes(body, ctx_stack, templates, content_html));
                        rendered = true;
                        break;
                    }
                }
                if !rendered && let Some(body) = &if_block.otherwise {
                    out.push_str(&render_nodes(body, ctx_stack, templates, content_html));
                }
            }

            Node::Forloop(forloop) => {
                let mut scope = create_scope(ctx_stack);
                if let Ok(arr) = engine.eval_with_scope::<Array>(&mut scope, &forloop.container) {
                    ctx_stack.push_scope();
                    for item in arr {
                        ctx_stack.set(forloop.value.clone(), dynamic_to_json(item));
                        out.push_str(&render_nodes(
                            &forloop.body,
                            ctx_stack,
                            templates,
                            content_html,
                        ));
                    }
                    ctx_stack.pop_scope();
                }
            }

            Node::Include(include) => {
                if let Some(partial_nodes) = templates.get(&include.path) {
                    let parent_rendered_content =
                        render_nodes(&include.body, ctx_stack, templates, None);

                    let empty_global: HashMap<String, Value> = HashMap::new();
                    let mut partial_stack = ContextStack::new(&empty_global);
                    partial_stack.push_scope();

                    for (k, raw_expr) in &include.local_ctx {
                        let mut scope = create_scope(ctx_stack);
                        if let Ok(val) = engine.eval_with_scope::<Dynamic>(&mut scope, raw_expr) {
                            partial_stack.set(k.clone(), dynamic_to_json(val));
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

fn create_scope<'a>(ctx_stack: &'a ContextStack) -> Scope<'a> {
    let mut scope = Scope::new();
    for (k, v) in ctx_stack.global {
        scope.push_dynamic(k.clone(), json_to_dynamic(v));
    }
    for scope_map in ctx_stack.scopes.iter() {
        for (k, v) in scope_map {
            scope.push_dynamic(k.clone(), json_to_dynamic(v));
        }
    }
    scope
}

fn json_to_dynamic(v: &Value) -> Dynamic {
    match v {
        Value::Array(arr) => {
            let mut rhai_arr = Array::new();
            for item in arr {
                rhai_arr.push(json_to_dynamic(item));
            }
            Dynamic::from(rhai_arr)
        }
        Value::Object(map) => {
            let mut rhai_map = rhai::Map::new();
            for (k, v) in map {
                rhai_map.insert(k.into(), json_to_dynamic(v));
            }
            Dynamic::from(rhai_map)
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        _ => Dynamic::UNIT,
    }
}

fn dynamic_to_json(d: Dynamic) -> Value {
    if d.is::<String>() {
        Value::String(d.cast::<String>())
    } else if d.is::<bool>() {
        Value::Bool(d.cast::<bool>())
    } else if d.is::<i64>() {
        Value::Number(d.cast::<i64>().into())
    } else if d.is::<f64>() {
        serde_json::json!(d.cast::<f64>())
    } else if d.is::<Array>() {
        Value::Array(d.cast::<Array>().into_iter().map(dynamic_to_json).collect())
    } else {
        Value::Null
    }
}

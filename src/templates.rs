use std::collections::HashMap;
use std::path::PathBuf;

use rhai::{AST, Engine, Module, Scope};
use serde_json::Value;

use crate::engine::{ContextStack, render_nodes};
use crate::nodes::Node;
use crate::parser::parse_template;

/// Stores parsed templates and precompiled scripts
pub struct Templates {
    templates: HashMap<String, Vec<Node>>, // relative_key -> nodes
    sources: HashMap<String, String>,      // relative_key -> absolute path
    pub(crate) script_asts: HashMap<String, Vec<AST>>, // relative_key -> precompiled ASTs
}

impl Templates {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            sources: HashMap::new(),
            script_asts: HashMap::new(),
        }
    }

    /// Load a single file under a specific relative key
    fn load_as(&mut self, absolute_path: &str, relative_key: &str) {
        let content = std::fs::read_to_string(absolute_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", absolute_path, e));
        let nodes = parse_template(&content);
        let key = normalize_key(relative_key);

        // Precompile scripts
        let engine = Engine::new();
        let asts = compile_scripts_for_nodes(&engine, &nodes);

        self.templates.insert(key.clone(), nodes);
        self.script_asts.insert(key.clone(), asts);
        self.sources.insert(key, absolute_path.to_string());
    }

    /// Load a file and derive its relative key by stripping a base dir
    fn load_under_base(&mut self, base_dir: &str, absolute_path: &str) {
        let rel = strip_base(base_dir, absolute_path);
        self.load_as(absolute_path, &rel);
    }

    /// Load using a glob, stripping the given `base_dir` from all matches.
    pub fn load_glob(&mut self, base_dir: &str, pattern: &str) {
        for entry in glob::glob(pattern).expect("Invalid glob pattern") {
            match entry {
                Ok(pathbuf) => {
                    let abs = pathbuf.to_string_lossy().to_string();
                    self.load_under_base(base_dir, &abs);
                }
                Err(e) => {
                    eprintln!("Glob error: {}", e);
                }
            }
        }
    }

    pub fn load_str(&mut self, key: &str, content: &str) {
        let nodes = parse_template(content);
        let rel_key = normalize_key(key);

        // Precompile scripts
        let engine = Engine::new();
        let asts = compile_scripts_for_nodes(&engine, &nodes);

        self.templates.insert(rel_key.clone(), nodes);
        self.script_asts.insert(rel_key.clone(), asts);
        self.sources.insert(rel_key, "<in-memory>".to_string());
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Vec<Node>> {
        self.templates.get(&normalize_key(key))
    }

    pub fn render_template(&self, path: &str, ctx: HashMap<String, Value>) -> String {
        if let Some(nodes) = self.get(path) {
            let mut ctx_stack = ContextStack::new(&ctx);
            let mut engine = Engine::new();

            // Register precompiled scripts for this template
            if let Some(asts) = self.script_asts.get(&normalize_key(path)) {
                for ast in asts {
                    let module = {
                        let eng_ref: &Engine = &engine;
                        match Module::eval_ast_as_new(Scope::new(), ast, eng_ref) {
                            Ok(m) => m,
                            Err(_e) => continue,
                        }
                    };
                    engine.register_global_module(module.into());
                }
            }

            render_nodes(nodes, &mut ctx_stack, self, None, &mut engine)
        } else {
            format!("<!-- Missing template: {} -->", path)
        }
    }
}

impl Default for Templates {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect all script bodies from nodes recursively
fn collect_script_bodies(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            Node::Script(code) => out.push(code.clone()),
            Node::Forloop(fl) => collect_script_bodies(&fl.body, out),
            Node::If(ifb) => {
                for (_, body) in &ifb.conditions {
                    collect_script_bodies(body, out);
                }
                if let Some(body) = &ifb.otherwise {
                    collect_script_bodies(body, out);
                }
            }
            Node::Include(inc) => {
                collect_script_bodies(&inc.body, out);
            }
            _ => {}
        }
    }
}

/// Compile all scripts in a template into ASTs
fn compile_scripts_for_nodes(engine: &Engine, nodes: &[Node]) -> Vec<AST> {
    let mut bodies = Vec::new();
    collect_script_bodies(nodes, &mut bodies);
    let mut asts = Vec::new();
    for code in bodies {
        if let Ok(ast) = engine.compile(&code) {
            asts.push(ast);
        } else {
            // Could log errors here
        }
    }
    asts
}

fn strip_base(base_dir: &str, absolute_path: &str) -> String {
    let base = std::fs::canonicalize(base_dir).unwrap_or_else(|_| PathBuf::from(base_dir));
    let abs = std::fs::canonicalize(absolute_path).unwrap_or_else(|_| PathBuf::from(absolute_path));
    let rel = pathdiff::diff_paths(&abs, &base).unwrap_or_else(|| PathBuf::from(absolute_path));

    normalize_key(rel.to_string_lossy().as_ref())
}

pub(crate) fn normalize_key<S: AsRef<str>>(s: S) -> String {
    let mut k = s.as_ref().replace('\\', "/");
    if let Some(stripped) = k.strip_prefix("./") {
        k = stripped.to_string();
    }
    k
}

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::engine::render_nodes;
use crate::nodes::Node;
use crate::parser::parse_template;

/// Stores parsed templates by a *relative key* like `partials/card.html`
pub struct Templates {
    templates: HashMap<String, Vec<Node>>, // relative_key -> nodes
    /// Optional: map relative_key -> absolute file path (useful for debugging or reloading)
    sources: HashMap<String, String>,
}

impl Templates {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// Load a single file under a specific relative key
    fn load_as(&mut self, absolute_path: &str, relative_key: &str) {
        let content = std::fs::read_to_string(absolute_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", absolute_path, e));
        let nodes = parse_template(&content);
        let key = normalize_key(relative_key);
        self.templates.insert(key.clone(), nodes);
        self.sources.insert(key, absolute_path.to_string());
    }

    /// Load a file and derive its relative key by stripping a base dir
    fn load_under_base(&mut self, base_dir: &str, absolute_path: &str) {
        let rel = strip_base(base_dir, absolute_path);
        self.load_as(absolute_path, &rel);
    }

    /// Load using a glob, stripping the given `base_dir` from all matches.
    ///
    /// Example:
    ///     templates.load_glob("templates", "templates/**/*.html");
    ///     // Keys become like "partials/card.html", "main.html"
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
        self.templates.insert(rel_key.clone(), nodes);
        self.sources.insert(rel_key, "<in-memory>".to_string());
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Vec<Node>> {
        self.templates.get(&normalize_key(key))
    }

    pub fn render_template(&self, path: &str, ctx: HashMap<String, Value>) -> String {
        if let Some(nodes) = self.get(path) {
            render_nodes(nodes, &ctx, self, None)
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

fn strip_base(base_dir: &str, absolute_path: &str) -> String {
    let base = std::fs::canonicalize(base_dir).unwrap_or_else(|_| PathBuf::from(base_dir));
    let abs = std::fs::canonicalize(absolute_path).unwrap_or_else(|_| PathBuf::from(absolute_path));
    let rel = pathdiff::diff_paths(&abs, &base).unwrap_or_else(|| PathBuf::from(absolute_path));

    normalize_key(rel.to_string_lossy().as_ref())
}

fn normalize_key<S: AsRef<str>>(s: S) -> String {
    let mut k = s.as_ref().replace('\\', "/");
    if let Some(stripped) = k.strip_prefix("./") {
        k = stripped.to_string();
    }
    k
}

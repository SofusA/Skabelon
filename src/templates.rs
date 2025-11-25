use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::engine::{ContextStack, render_nodes};
use crate::nodes::Node;
use crate::parser::parse_template;

#[derive(Default)]
pub struct Templates {
    templates: HashMap<String, Vec<Node>>,
    glob: Option<String>,
}

impl Templates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reload(&mut self) {
        if let Some(glob) = self.glob.clone() {
            self.load_glob(&glob);
        };
    }

    /// Load a single file under a specific relative key
    fn load_as(&mut self, absolute_path: &str, relative_key: &str) {
        let content = std::fs::read_to_string(absolute_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", absolute_path, e));
        let nodes = parse_template(&content);
        let key = normalize_key(relative_key);
        self.templates.insert(key.clone(), nodes);
    }

    /// Load using a glob, stripping the base directory from all matches.
    ///
    /// Example:
    ///     templates.load_glob("templates/**/*.html");
    ///     // Keys become like "partials/card.html", "main.html"
    pub fn load_glob(&mut self, pattern: &str) {
        self.glob = Some(pattern.into());

        let base_dir = derive_base_dir(pattern);

        for entry in glob::glob(pattern).expect("Invalid glob pattern") {
            match entry {
                Ok(pathbuf) => {
                    let abs = pathbuf.to_string_lossy().to_string();
                    {
                        let rel = strip_base(&base_dir, &abs);
                        self.load_as(&abs, &rel);
                    };
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
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Vec<Node>> {
        self.templates.get(&normalize_key(key))
    }

    pub fn render_template(&self, path: &str, ctx: HashMap<String, Value>) -> String {
        if let Some(nodes) = self.get(path) {
            let mut ctx_stack = ContextStack::new(&ctx);
            render_nodes(nodes, &mut ctx_stack, self, None)
        } else {
            format!("<!-- Missing template: {} -->", path)
        }
    }
}

fn strip_base(base_dir: &str, absolute_path: &str) -> String {
    let base = std::fs::canonicalize(base_dir).unwrap_or_else(|_| PathBuf::from(base_dir));
    let abs = std::fs::canonicalize(absolute_path).unwrap_or_else(|_| PathBuf::from(absolute_path));
    let rel = diff_paths(&abs, &base).unwrap_or_else(|| PathBuf::from(absolute_path));

    normalize_key(rel.to_string_lossy().as_ref())
}

fn normalize_key<S: AsRef<str>>(s: S) -> String {
    let mut k = s.as_ref().replace('\\', "/");
    if let Some(stripped) = k.strip_prefix("./") {
        k = stripped.to_string();
    }
    k
}

fn derive_base_dir(pattern: &str) -> String {
    // Find first '*' and take everything before it
    if let Some(idx) = pattern.find('*') {
        let base = &pattern[..idx];
        // Remove trailing slash if present
        let base = base.trim_end_matches('/');
        base.to_string()
    } else {
        // No wildcard? Use parent directory
        let path = std::path::Path::new(pattern);
        path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_string_lossy()
            .to_string()
    }
}

pub fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut it_a = path.components();
        let mut it_b = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (it_a.next(), it_b.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(it_a.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(Component::CurDir)) => comps.push(a),
                (Some(_), Some(Component::ParentDir)) => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in it_b {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(it_a.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

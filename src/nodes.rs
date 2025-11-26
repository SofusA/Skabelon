use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Text(String),
    VariableBlock(Vec<String>),
    Forloop(ForLoop),
    If(If),
    Include(Include),
    ContentPlaceholder,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForLoop {
    pub value: String,
    pub container: Vec<String>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct If {
    pub conditions: Vec<(Vec<String>, Vec<Node>)>,
    pub otherwise: Option<Vec<Node>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Include {
    pub path: String,
    pub body: Vec<Node>,
    pub local_ctx: Vec<(String, Value)>,
}

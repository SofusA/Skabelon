#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Text(String),
    VariableBlock(String),
    Forloop(ForLoop),
    If(If),
    Include(Include),
    ContentPlaceholder,
    Script(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ForLoop {
    pub value: String,
    pub container: String,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct If {
    pub conditions: Vec<(String, Vec<Node>)>,
    pub otherwise: Option<Vec<Node>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Include {
    pub path: String,
    pub body: Vec<Node>,
    pub local_ctx: Vec<(String, String)>,
}

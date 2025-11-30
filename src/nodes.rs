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
pub enum Operand {
    Path(Vec<String>),
    Literal(Value),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    Literal(bool),
    Path(Vec<String>),
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Compare {
        left: Operand,
        op: CompareOp,
        right: Operand,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct If {
    pub conditions: Vec<(Condition, Vec<Node>)>,
    pub otherwise: Option<Vec<Node>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Include {
    pub path: String,
    pub body: Vec<Node>,
    pub local_ctx: Vec<(String, LocalValue)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LocalValue {
    Literal(Value),
    Path(Vec<String>),
}

use std::collections::HashMap;

use serde_json::{Value, json};
use skabelon::Templates;

#[test]
fn if_condition() {
    let template_str = "@if(value) {hello} @if(other) {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let mut ctx = HashMap::new();
    ctx.insert("value".to_string(), Value::Bool(true));
    ctx.insert("other".to_string(), Value::Bool(false));

    let output = templates.render_template("template", ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn if_else() {
    let template_str = "@if(value) {hello} @else {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let mut ctx = HashMap::new();
    ctx.insert("value".to_string(), Value::Bool(false));

    let output = templates.render_template("template", ctx);

    let expected = "world";

    assert_eq!(output, expected);
}

#[test]
fn if_else_if() {
    let template = "@if(a) {A} @else if(b) {B} @else {C}";

    let mut templates = Templates::new();
    templates.load_str("template", template);

    let mut ctx = HashMap::new();
    ctx.insert("a".to_string(), json!(false));
    ctx.insert("b".to_string(), json!(true));

    let output = templates.render_template("template", ctx);
    assert_eq!(output, "B");
}

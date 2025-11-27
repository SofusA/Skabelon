use serde_json::json;
use skabelon::Templates;

#[test]
fn if_condition() {
    let template_str = "@if(value) {hello} @if(other) {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value": true, "other": false});

    let output = templates.render_template("template", ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn if_else() {
    let template_str = "@if(value) {hello} @else {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value": false});

    let output = templates.render_template("template", ctx);

    let expected = "world";

    assert_eq!(output, expected);
}

#[test]
fn if_else_if() {
    let template = "@if(a) {A} @else if(b) {B} @else {C}";

    let mut templates = Templates::new();
    templates.load_str("template", template);

    let ctx = json!({"a": false, "b": true});

    let output = templates.render_template("template", ctx);
    assert_eq!(output, "B");
}

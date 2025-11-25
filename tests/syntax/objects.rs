use std::collections::HashMap;

use serde_json::json;
use skabelon::Templates;

#[test]
fn values_are_parsed() {
    let template_str = "{{number}} {{bool}} {{string}}";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let mut ctx = HashMap::new();
    ctx.insert("number".to_string(), json!(1));
    ctx.insert("bool".to_string(), json!(true));
    ctx.insert("string".to_string(), json!("hello"));

    let output = templates.render_template("test", ctx);

    let expected = "1 true hello";

    assert_eq!(output, expected);
}

#[test]
fn objects_are_parsed() {
    let template_str = "{{object1[\"value\"]}} {{object2.number}}";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let mut ctx = HashMap::new();
    ctx.insert("object1".to_string(), json!({"value": "hello"}));
    ctx.insert("object2".to_string(), json!({"number": 1}));

    let output = templates.render_template("test", ctx);

    let expected = "hello 1";

    assert_eq!(output, expected);
}

#[test]
fn none_objects_values_are_false() {
    let template = "@if(object[\"value\"]) {Hello World}";

    let mut templates = Templates::new();
    templates.load_str("template", template);

    let mut ctx = HashMap::new();
    ctx.insert("object".to_string(), json!({"value": None::<String>}));
    let output = templates.render_template("template", ctx);

    let expected = "";

    assert_eq!(output, expected);
}

#[test]
fn objects_values_are_truthy() {
    let template = "@if(object[\"value\"]) {{{object[\"value\"}}}";

    let mut templates = Templates::new();
    templates.load_str("template", template);

    let mut ctx = HashMap::new();
    ctx.insert("object".to_string(), json!({"value": Some("Hello world")}));
    let output = templates.render_template("template", ctx);

    let expected = "Hello world";

    assert_eq!(output, expected);
}

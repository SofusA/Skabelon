use serde_json::json;
use skabelon::Templates;

#[test]
fn values_are_parsed() {
    let template_str = "{{arr[0]}} {{arr[1]}} {{arr[2]}} {{arr[4]}}";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let ctx = json!({"arr": ["A", "B", "C"]});

    let output = templates.render("test", &ctx);

    let expected = "A B C";

    assert_eq!(output.trim(), expected);
}

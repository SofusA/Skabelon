use serde_json::json;
use skabelon::Templates;

#[test]
fn if_condition() {
    let template_str = "@if(value) {hello} @if(other) {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value": true, "other": false});

    let output = templates.render("template", &ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn and() {
    let template_str = "@if(value1 && value2) {hello} @if(value1 and value3) {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": true, "value2": true, "value3": false});

    let output = templates.render("template", &ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn and_multi() {
    let template_str = "@if(value1 && value2 && value3) {hello}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": true, "value2": true, "value3": true});

    let output = templates.render("template", &ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn and_multi_2() {
    let template_str = "@if(value1 && value2 || value3) {hello}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": true, "value2": false, "value3": true});

    let output = templates.render("template", &ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn or() {
    let template_str = "@if(value1 || value2) {hello} @if(value2 or value3) {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": true, "value2": false, "value3": false});

    let output = templates.render("template", &ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn if_else() {
    let template_str = "@if(value) {hello} @else {world}";

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value": false});

    let output = templates.render("template", &ctx);

    let expected = "world";

    assert_eq!(output, expected);
}

#[test]
fn if_else_if() {
    let template = "@if(a) {A} @else if(b) {B} @else {C}";

    let mut templates = Templates::new();
    templates.load_str("template", template);

    let ctx = json!({"a": false, "b": true});

    let output = templates.render("template", &ctx);
    assert_eq!(output, "B");
}

#[test]
fn if_variable() {
    let template_str = r#"@if(value1 == "A") {hello } @if(value3 == 1) {world}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "B", "value3": 1});

    let output = templates.render("template", &ctx);

    let expected = "hello world";

    assert_eq!(output, expected);
}

#[test]
fn if_variable_2() {
    let template_str =
        r#"@if(value1 == "A") {hello } @if(value2 != "B") {world} @if(value3 < 10) {world}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "B", "value3": 5});

    let output = templates.render("template", &ctx);

    let expected = "hello world";

    assert_eq!(output, expected);
}

#[test]
fn if_variable_3() {
    let template_str = r#"@if(value1 == value2) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "A"});

    let output = templates.render("template", &ctx);

    let expected = "hello";

    assert_eq!(output, expected);
}

#[test]
fn if_variable_eq_string_literal() {
    let template_str = r#"@if(value1 == "A") {hello } @if(value3 == 1) {world}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "B", "value3": 1});

    let output = templates.render("template", &ctx);

    let expected = "hello world";
    assert_eq!(output, expected);
}

#[test]
fn if_variable_ne_string_literal_and_lt_number_literal() {
    let template_str =
        r#"@if(value1 == "A") {hello } @if(value2 != "B") {world} @if(value3 < 10) {world}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "B", "value3": 5});

    let output = templates.render("template", &ctx);

    let expected = "hello world";
    assert_eq!(output, expected);
}

#[test]
fn if_variable_eq_variable() {
    let template_str = r#"@if(value1 == value2) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "A"});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_variable_ne_variable() {
    let template_str = r#"@if(value1 != value2) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "A", "value2": "B"});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_number_comparisons_literals() {
    let template_str =
        r#"@if(num1 < 10) {a } @if(num2 > 5) {b } @if(num3 <= 3) {c } @if(num4 >= 7) {d}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"num1": 9, "num2": 6, "num3": 3, "num4": 7});

    let output = templates.render("template", &ctx);

    let expected = "a b c d";
    assert_eq!(output, expected);
}

#[test]
fn if_number_comparisons_variable_to_variable() {
    let template_str = r#"@if(a < b) {x } @if(b > c) {y } @if(c <= d) {z } @if(d >= e) {w}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"a": 1, "b": 2, "c": 1, "d": 1, "e": 1});

    let output = templates.render("template", &ctx);

    let expected = "x y z w";
    assert_eq!(output, expected);
}

#[test]
fn if_string_ordering() {
    let template_str = r#"@if(val1 < val2) {l } @if(val2 > val1) {g } @if(val1 <= val1) {le } @if(val2 >= val2) {ge}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    // Lexicographic comparisons
    let ctx = json!({"val1": "A", "val2": "B"});

    let output = templates.render("template", &ctx);

    let expected = "l g le ge";
    assert_eq!(output, expected);
}

#[test]
fn if_boolean_literals_and_variables() {
    let template_str =
        r#"@if(flag1 == true) {T1 } @if(flag2 == false) {T2 } @if(flag3 != true) {T3}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({ "flag1": true, "flag2": false, "flag3": false });

    let output = templates.render("template", &ctx);

    let expected = "T1 T2 T3";
    assert_eq!(output, expected);
}

#[test]
fn if_mixed_types_equality() {
    let template_str = r#"@if(str1 == "10") {S } @if(num1 == 10) {N } @if(str1 != num1) {M}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    // str1 (string) vs num1 (number)
    let ctx = json!({ "str1": "10", "num1": 10 });

    let output = templates.render("template", &ctx);

    // Our compare_values handles Eq for mixed by Value equality;
    // String("10") != Number(10), so S and N are true individually, M is true because mixed !=
    let expected = "S N M";
    assert_eq!(output, expected);
}

#[test]
fn if_and_or_precedence() {
    // and binds tighter than or
    let template_str = r#"@if(value1 and value2 or value3) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    // value1 and value2 => true && false = false; false or true = true => hello
    let ctx = json!({"value1": true, "value2": false, "value3": true});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_parentheses_precedence() {
    // parentheses change precedence
    let template_str = r#"@if((value1 or value2) and value3) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    // (true or false) and true => true and true => true
    let ctx = json!({"value1": true, "value2": false, "value3": true});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_unary_not_simple() {
    let template_str = r#"@if(!value) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value": false});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_unary_not_with_comparison() {
    let template_str = r#"@if(!(value1 == "A")) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": "B"});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_unary_not_and_or_combo() {
    let template_str = r#"@if(!value1 and (value2 == "X" or value3 == "Y")) {hello}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({"value1": false, "value2": "Z", "value3": "Y"});

    let output = templates.render("template", &ctx);

    let expected = "hello";
    assert_eq!(output, expected);
}

#[test]
fn if_variable_vs_literal_number_edge() {
    let template_str = r#"@if(n == 0) {z } @if(n != 0) {nz } @if(n > -1) {gt } @if(n >= 0) {ge}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({ "n": 0 });

    let output = templates.render("template", &ctx);

    let expected = "z gt ge";
    assert_eq!(output, expected);
}

#[test]
fn if_variable_vs_variable_booleans() {
    let template_str = r#"@if(a == b) {eq } @if(a != b) {ne}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({ "a": true, "b": true });

    let output = templates.render("template", &ctx);

    let expected = "eq ";
    assert_eq!(output, expected);
}

#[test]
fn if_mixed_numeric_float_int() {
    let template_str = r#"@if(f == 1.1) {eqf } @if(i == 1) {eqi } @if(f == i) {eqm}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({ "f": 1.1, "i": 1 });

    let output = templates.render("template", &ctx);

    // eqf true, eqi true, eqm false (mixed types not equal by Value)
    let expected = "eqf eqi ";
    assert_eq!(output, expected);
}

#[test]
fn if_string_equality_variable_to_literal_quotes() {
    let template_str = r#"@if(name == "Alice") {hi } @if(name != "Bob") {notbob}"#;

    let mut templates = Templates::new();
    templates.load_str("template", template_str);

    let ctx = json!({ "name": "Alice" });

    let output = templates.render("template", &ctx);

    let expected = "hi notbob";
    assert_eq!(output, expected);
}

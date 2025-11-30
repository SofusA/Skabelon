use serde_json::json;
use skabelon::Templates;
use std::time::Instant;

mod syntax;

#[test]
fn test() {
    let template_str = "<h1>Hello</h1>@if(show) {<p>Visible!</p>}@if(false) {<p>Hidden!</p>}@for(item in items) {<span>{{item}}</span>}";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let ctx = json!({"show": true, "items": ["A", "B", "C"]});

    let output = templates.render("test", &ctx);

    let expected = "<h1>Hello</h1><p>Visible!</p><span>A</span><span>B</span><span>C</span>";

    assert_eq!(output, expected);
}

#[test]
fn support_emoji() {
    let template_str = "Hi ☺️";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let output = templates.render("test", &json!({}));

    let expected = "Hi ☺️";

    assert_eq!(output, expected);
}

#[test]
fn whites_space_test() {
    let template_str = r#"
<h1>Testing template</h1>

<h2>If statements</h2>
@if (true) {
  <span>hello</span>
}
"#;

    let expected = r#"
<h1>Testing template</h1>

<h2>If statements</h2>

  <span>hello</span>
"#;

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let output = templates.render("test", Default::default());

    assert_eq!(output, expected);
}

#[test]
fn big_test() {
    let main_template = include_str!("templates/main.html");
    let partial_1_template = include_str!("templates/partial1.html");
    let partial_2_template = include_str!("templates/partial2.html");
    let partial_3_template = include_str!("templates/partial3.html");

    let mut templates = Templates::new();
    templates.load_str("main.html", main_template);
    templates.load_str("partial1.html", partial_1_template);
    templates.load_str("partial2.html", partial_2_template);
    templates.load_str("partial3.html", partial_3_template);

    let object = json!({"true": true, "false": false, "number": 5, "string": "world", "none": None::<String>, "array": [1, 2, 3]});

    let ctx = json!({"bool_true": true,"bool_false": false, "array": ["A", "B", "C"], "string": "hello", "object": object });

    let output = templates
        .render("main.html", &ctx)
        .replace("\n", "")
        .replace("  ", "");

    let expected = include_str!("templates/expected.html").replace("\n", "");

    assert_eq!(output, expected);
}

#[test]
fn big_table() {
    const SIZE: usize = 100;

    let mut table = Vec::with_capacity(SIZE);
    for _ in 0..SIZE {
        let mut inner = Vec::with_capacity(SIZE);
        for i in 0..SIZE {
            inner.push(i);
        }
        table.push(inner);
    }

    let template_str =
        "<table>@for(row in table) {<tr>@for(col in row) {<td>{{col}}</td>}</tr>}</table>";
    let mut templates = Templates::new();
    templates.load_str("big-table", template_str);

    let ctx = json!({"table": table});

    let timer = Instant::now();
    let output = templates.render("big-table", &ctx);

    let elapsed = timer.elapsed();
    println!("elapsed micros: {}", elapsed.subsec_micros());

    let mut expected = "<table>".to_string();
    for row in table {
        expected += "<tr>";
        for col in row {
            expected = expected + &format!("<td>{col}</td>");
        }
        expected += "</tr>";
    }
    expected += "</table>";

    assert_eq!(output, expected);
}

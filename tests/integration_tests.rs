use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use template::templates::Templates;

#[test]
fn integration_render_with_glob_and_relative_keys() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("templates");
    let partials = base.join("partials");
    std::fs::create_dir_all(&partials).unwrap();

    let main_path = base.join("main.html");
    let partial_path = partials.join("card.html");

    fs::write(
        &main_path,
        r#"
        <h1>{{title}}</h1>
        @include(partials/card.html) {<p>{{body}}</p>}
    "#,
    )
    .unwrap();

    fs::write(
        &partial_path,
        r#"
        <div>@content</div>
    "#,
    )
    .unwrap();

    let mut templates = Templates::new();
    templates.load_glob(
        base.to_str().unwrap(),
        &format!("{}/**/*.html", base.to_str().unwrap()),
    );
    let mut ctx = HashMap::new();
    ctx.insert("title".to_string(), json!("Hello World"));
    ctx.insert("body".to_string(), json!("This is body"));

    let output = templates.render_template("main.html", ctx);
    let expected = r#"
        <h1>Hello World</h1>
        <div><p>This is body</p></div>
    "#;

    assert_eq!(normalize_ws(&output), normalize_ws(expected));
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn big_table() {
    const SIZE: usize = 2;

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

    let mut ctx = HashMap::new();
    ctx.insert("table".to_string(), json!(table));

    let timer = Instant::now();
    let output = templates.render_template("big-table", ctx);

    let elapsed = timer.elapsed();
    println!("elapsed micros: {}", elapsed.subsec_micros());

    let mut expected = "<table>".to_string();
    for row in table {
        expected += "<tr>";
        for col in row {
            expected += format!("<td>{col}</td>").as_str();
        }
        expected += "</tr>";
    }
    expected += "</table>";

    assert_eq!(output, expected);
}

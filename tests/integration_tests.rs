use serde_json::json;
use skabelon::Templates;
use std::collections::HashMap;
use std::time::Instant;

mod syntax;

#[test]
fn test() {
    let template_str = "<h1>Hello</h1>@if(show) {<p>Visible!</p>}@if(false) {<p>Hidden!</p>}@for(item in items) {<span>{{item}}</span>}";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let mut ctx = HashMap::new();
    ctx.insert("show".to_string(), json!(true));
    ctx.insert("items".to_string(), json!(["A", "B", "C"]));

    let output = templates.render_template("test", ctx);

    let expected = "<h1>Hello</h1><p>Visible!</p><span>A</span><span>B</span><span>C</span>";

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
            expected = expected + &format!("<td>{col}</td>");
        }
        expected += "</tr>";
    }
    expected += "</table>";

    assert_eq!(output, expected);
}

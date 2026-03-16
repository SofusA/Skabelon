use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
use skabelon::Templates;

fn bench_big_table(c: &mut Criterion) {
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

    let ctx = json!({ "table": table });

    let mut expected = "<table>".to_string();
    for row in ctx["table"].as_array().unwrap() {
        expected.push_str("<tr>");
        for col in row.as_array().unwrap() {
            expected.push_str(&format!("<td>{}</td>", col));
        }
        expected.push_str("</tr>");
    }
    expected.push_str("</table>");

    c.bench_function("render_big_table", |b| {
        b.iter(|| {
            let out = templates.render("big-table", &ctx);
            assert_eq!(out, expected);
        })
    });
}

fn config() -> Criterion {
    Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(10))
}

criterion_group! {
    name = benches;
    config = config();
    targets = bench_big_table
}

criterion_main!(benches);

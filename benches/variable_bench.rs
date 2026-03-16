use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
use skabelon::Templates;

fn bench_variables_test(c: &mut Criterion) {
    let main_template = include_str!("../tests/templates/main.html");
    let partial_1_template = include_str!("../tests/templates/partial1.html");
    let partial_2_template = include_str!("../tests/templates/partial2.html");
    let partial_3_template = include_str!("../tests/templates/partial3.html");

    let mut templates = Templates::new();
    templates.load_str("main.html", main_template);
    templates.load_str("partial1.html", partial_1_template);
    templates.load_str("partial2.html", partial_2_template);
    templates.load_str("partial3.html", partial_3_template);

    let object = json!({
        "true": true,
        "false": false,
        "number": 5,
        "string": "world",
        "none": serde_json::Value::Null,
        "array": [1, 2, 3]
    });

    let ctx = json!({
        "bool_true": true,
        "bool_false": false,
        "array": ["A", "B", "C"],
        "string": "hello",
        "object": object
    });

    let expected = include_str!("../tests/templates/expected.html")
        .replace('\n', "")
        .replace("  ", "");

    c.bench_function("render_big_test", |b| {
        b.iter(|| {
            let output = templates
                .render("main.html", &ctx)
                .replace('\n', "")
                .replace("  ", "");

            assert_eq!(output, expected);

            black_box(output);
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
    targets = bench_variables_test
}

criterion_main!(benches);

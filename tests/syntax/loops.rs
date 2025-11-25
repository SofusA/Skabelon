use std::collections::HashMap;

use serde_json::json;
use skabelon::Templates;

#[test]
fn for_loops() {
    let template_str = "@for(item in items) {<span>{{item.index}}: {{item.value}}</span>}";

    let mut templates = Templates::new();
    templates.load_str("test", template_str);

    let arr = ["A", "B", "C"];
    let arr: Vec<_> = arr
        .into_iter()
        .enumerate()
        .map(|x| json!({"index": x.0+1, "value": x.1}))
        .collect();

    let mut ctx = HashMap::new();
    ctx.insert("items".to_string(), json!(arr));

    let output = templates.render_template("test", ctx);

    let expected = "<span>1: A</span><span>2: B</span><span>3: C</span>";

    assert_eq!(output, expected);
}

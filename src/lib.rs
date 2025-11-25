mod engine;
mod nodes;
mod parser;
mod templates;

pub use templates::Templates;

#[cfg(test)]
mod tests {
    use crate::templates::Templates;
    use serde_json::{Value, json};
    use std::collections::HashMap;

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

    #[test]
    fn if_condition() {
        let template_str = "@if(value) {hello} @if(other) {world}";

        let mut templates = Templates::new();
        templates.load_str("template", template_str);

        let mut ctx = HashMap::new();
        ctx.insert("value".to_string(), Value::Bool(true));
        ctx.insert("other".to_string(), Value::Bool(false));

        let output = templates.render_template("template", ctx);

        let expected = "hello";

        assert_eq!(output, expected);
    }

    #[test]
    fn if_else() {
        let template_str = "@if(value) {hello} @else {world}";

        let mut templates = Templates::new();
        templates.load_str("template", template_str);

        let mut ctx = HashMap::new();
        ctx.insert("value".to_string(), Value::Bool(false));

        let output = templates.render_template("template", ctx);

        let expected = "world";

        assert_eq!(output, expected);
    }

    #[test]
    fn if_else_if() {
        let template = "@if(a) {A} @else if(b) {B} @else {C}";

        let mut templates = Templates::new();
        templates.load_str("template", template);

        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), json!(false));
        ctx.insert("b".to_string(), json!(true));

        let output = templates.render_template("template", ctx);
        assert_eq!(output, "B");
    }

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
    fn partial() {
        let parent = "@include(partial) {} @if(true) {<span>World</span>}";
        let partial = "<span>Hello</span>";

        let mut templates = Templates::new();
        templates.load_str("parent", parent);
        templates.load_str("partial", partial);

        let ctx = HashMap::new();
        let output = templates.render_template("parent", ctx);

        let expected = "<span>Hello</span> <span>World</span>";

        assert_eq!(output, expected);
    }

    #[test]
    fn partial_no_block() {
        let parent = "@include(partial) @if(true) {<span>World</span>}";
        let partial = "<span>Hello</span>";

        let mut templates = Templates::new();
        templates.load_str("parent", parent);
        templates.load_str("partial", partial);

        let ctx = HashMap::new();
        let output = templates.render_template("parent", ctx);

        let expected = "<span>Hello</span><span>World</span>";

        assert_eq!(output, expected);
    }

    #[test]
    fn partial_block() {
        let parent = "@include(partial) {Hello World}";
        let partial = "<div>@content</div>";

        let mut templates = Templates::new();
        templates.load_str("parent", parent);
        templates.load_str("partial", partial);

        let ctx = HashMap::new();
        let output = templates.render_template("parent", ctx);

        let expected = "<div>Hello World</div>";

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

    #[test]
    fn partial_separated_context() {
        let parent =
            "{{value}}{{parent_value}} @include(partial; value='hello') {Hello {{parent_value}}}";
        let partial = "{{value}} @content{{parent_value}}";

        let mut templates = Templates::new();
        templates.load_str("parent", parent);
        templates.load_str("partial", partial);

        let mut ctx = HashMap::new();
        ctx.insert("parent_value".to_string(), json!("World"));

        let output = templates.render_template("parent", ctx);

        let expected = "World hello Hello World";

        assert_eq!(output, expected);
    }

    #[test]
    fn partial_with_context() {
        let parent = "@include(partial; partial_var=\"partial\") {<span>{{parent_var}}</span>}";
        let partial = "<div>{{partial_var}} @content</div>";

        let mut templates = Templates::new();
        templates.load_str("parent", parent);
        templates.load_str("partial", partial);

        let mut ctx = HashMap::new();
        ctx.insert("parent_var".to_string(), json!("parent"));
        ctx.insert("partial_var".to_string(), json!("partial"));

        let output = templates.render_template("parent", ctx);

        let expected = "<div>partial <span>parent</span></div>";

        assert_eq!(output, expected);
    }
}

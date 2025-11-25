mod engine;
mod nodes;
mod parser;
pub mod templates;

#[cfg(test)]
mod tests {
    use crate::templates::Templates;
    use serde_json::json;
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
        let template_str = "{{object1[\"value\"]}} {{object2[\"number\"}}";

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

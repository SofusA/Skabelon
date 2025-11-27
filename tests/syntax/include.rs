use serde_json::json;
use skabelon::Templates;

#[test]
fn partial() {
    let parent = "@include(partial) {} @if(true) {<span>World</span>}";
    let partial = "<span>Hello</span>";

    let mut templates = Templates::new();
    templates.load_str("parent", parent);
    templates.load_str("partial", partial);

    let output = templates.render("parent", Default::default());

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

    let output = templates.render("parent", Default::default());

    let expected = "<span>Hello</span><span>World</span>";

    assert_eq!(output, expected);
}

#[test]
fn partial_block() {
    let parent = "@include(partial) {Hello World}";
    let partial = "<div><content-slot></div>";

    let mut templates = Templates::new();
    templates.load_str("parent", parent);
    templates.load_str("partial", partial);

    let output = templates.render("parent", Default::default());

    let expected = "<div>Hello World</div>";

    assert_eq!(output, expected);
}

#[test]
fn partial_separated_context() {
    let parent =
        "{{value}}{{parent_value}} @include(partial; value='hello') {Hello {{parent_value}}}";
    let partial = "{{value}} <content-slot>{{parent_value}}";

    let mut templates = Templates::new();
    templates.load_str("parent", parent);
    templates.load_str("partial", partial);

    let ctx = json!({"parent_value": "World"});

    let output = templates.render("parent", &ctx);

    let expected = "World hello Hello World";

    assert_eq!(output, expected);
}

#[test]
fn partial_with_context() {
    let parent = "@include(partial; partial_var=\"partial\") {<span>{{parent_var}}</span>}";
    let partial = "<div>{{partial_var}} <content-slot></div>";

    let mut templates = Templates::new();
    templates.load_str("parent", parent);
    templates.load_str("partial", partial);

    let ctx = json!({"parent_var": "parent", "partial_var": "partial"});

    let output = templates.render("parent", &ctx);

    let expected = "<div>partial <span>parent</span></div>";

    assert_eq!(output, expected);
}

#[test]
fn partial_with_variable_context() {
    let parent = "@include(partial; partial_var=variable) {world}";
    let partial = "{{partial_var}} world";

    let mut templates = Templates::new();
    templates.load_str("parent", parent);
    templates.load_str("partial", partial);

    let ctx = json!({"variable": "hello"});

    let output = templates.render("parent", &ctx);

    let expected = "hello world";

    assert_eq!(output, expected);
}

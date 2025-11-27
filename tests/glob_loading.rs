use serde_json::json;
use skabelon::Templates;
use std::fs;

#[test]
fn render_with_glob_and_relative_keys() {
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
        <div>{{ content }}</div>
    "#,
    )
    .unwrap();

    let mut templates = Templates::new();
    templates.load_glob(&format!("{}/**/*.html", base.to_str().unwrap()));
    let ctx = json!({"title": "Hello World", "body": "This is body"});

    let output = templates.render("main.html", &ctx);
    let expected = r#"
        <h1>Hello World</h1>
        <div><p>This is body</p></div>
    "#;

    assert_eq!(normalize_ws(&output), normalize_ws(expected));
}

#[test]
fn reload_glob() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("templates");
    std::fs::create_dir_all(&base).unwrap();
    let main_path = base.join("main.html");
    fs::write(&main_path, "hello").unwrap();
    let mut templates = Templates::new();

    templates.load_glob(&format!("{}/**/*.html", base.to_str().unwrap()));

    let output = templates.render("main.html", Default::default());
    let expected = "hello";
    assert_eq!(normalize_ws(&output), normalize_ws(expected));

    fs::write(&main_path, "world").unwrap();
    templates.reload();
    let output = templates.render("main.html", Default::default());

    let expected = "world";
    assert_eq!(normalize_ws(&output), normalize_ws(expected));
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

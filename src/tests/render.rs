use cargo_registry::render;

#[test]
fn empty_text() {
    let text = "";
    let result = render::markdown_to_html(text);
    assert_eq!(result.is_ok(), true);
    let rendered = result.unwrap();
    assert_eq!(rendered, "");
}

#[test]
fn text_with_script_tag() {
    let text = "foo_readme\n\n<script>alert('Hello World')</script>";
    let result = render::markdown_to_html(text);
    assert_eq!(result.is_ok(), true);
    let rendered = result.unwrap();
    assert_eq!(rendered.contains("foo_readme"), true);
    assert_eq!(rendered.contains("script"), false);
    assert_eq!(rendered.contains("alert('Hello World')"), true);
}

#[test]
fn text_with_iframe_tag() {
    let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
    let result = render::markdown_to_html(text);
    assert_eq!(result.is_ok(), true);
    let rendered = result.unwrap();
    assert_eq!(rendered.contains("foo_readme"), true);
    assert_eq!(rendered.contains("iframe"), false);
    assert_eq!(rendered.contains("alert('Hello World')"), true);
}

#[test]
fn text_with_unknwon_tag() {
    let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
    let result = render::markdown_to_html(text);
    assert_eq!(result.is_ok(), true);
    let rendered = result.unwrap();
    assert_eq!(rendered.contains("foo_readme"), true);
    assert_eq!(rendered.contains("unknown"), false);
    assert_eq!(rendered.contains("alert('Hello World')"), true);
}

#[test]
fn text_with_inline_javascript() {
    let text = r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
    let result = render::markdown_to_html(text);
    assert_eq!(result.is_ok(), true);
    let rendered = result.unwrap();
    assert_eq!(rendered.contains("foo_readme"), true);
    assert_eq!(rendered.contains("<a"), true);
    assert_eq!(rendered.contains("href="), true);
    assert_eq!(rendered.contains("onclick"), false);
    assert_eq!(rendered.contains("window.alert"), false);
    assert_eq!(rendered.contains("Crate page"), true);
}

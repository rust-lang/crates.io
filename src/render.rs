use ammonia::Ammonia;
use comrak;

use util::CargoResult;

/// Context for markdown to HTML rendering.
#[allow(missing_debug_implementations)]
pub struct MarkdownRenderer<'a> {
    html_sanitizer: Ammonia<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates a new renderer instance.
    pub fn new() -> MarkdownRenderer<'a> {
        let tags = [
            "a",
            "b",
            "blockquote",
            "br",
            "code",
            "dd",
            "del",
            "dl",
            "dt",
            "em",
            "h1",
            "h2",
            "h3",
            "hr",
            "i",
            "img",
            "input",
            "kbd",
            "li",
            "ol",
            "p",
            "pre",
            "s",
            "strike",
            "strong",
            "sub",
            "sup",
            "table",
            "tbody",
            "td",
            "th",
            "thead",
            "tr",
            "ul",
            "hr",
            "span",
        ].iter()
            .cloned()
            .collect();
        let tag_attributes = [
            ("a", ["href", "target"].iter().cloned().collect()),
            (
                "img",
                ["width", "height", "src", "alt", "align", "width"]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                "input",
                ["checked", "disabled", "type"].iter().cloned().collect(),
            ),
        ].iter()
            .cloned()
            .collect();
        let html_sanitizer = Ammonia {
            keep_cleaned_elements: true,
            tags: tags,
            tag_attributes: tag_attributes,
            ..Ammonia::default()
        };
        MarkdownRenderer { html_sanitizer: html_sanitizer }
    }

    /// Renders the given markdown to HTML using the current settings.
    pub fn to_html(&self, text: &str) -> CargoResult<String> {
        let mut options = comrak::ComrakOptions::default();
        options.ext_tasklist = true;
        let rendered = comrak::markdown_to_html(text, &options);
        Ok(self.html_sanitizer.clean(&rendered))
    }
}

impl<'a> Default for MarkdownRenderer<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders a markdown text to sanitized HTML.
///
/// The returned text should not contain any harmful HTML tag or attribute (such as iframe,
/// onclick, onmouseover, etc.).
///
/// # Examples
///
/// ```
/// use render::markdown_to_html;
///
/// let text = "[Rust](https://rust-lang.org/) is an awesome *systems programming* language!";
/// let rendered = markdown_to_html(text)?;
/// ```
pub fn markdown_to_html(text: &str) -> CargoResult<String> {
    let renderer = MarkdownRenderer::new();
    renderer.to_html(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text() {
        let text = "";
        let result = markdown_to_html(text);
        assert_eq!(result.is_ok(), true);
        let rendered = result.unwrap();
        assert_eq!(rendered, "");
    }

    #[test]
    fn text_with_script_tag() {
        let text = "foo_readme\n\n<script>alert('Hello World')</script>";
        let result = markdown_to_html(text);
        assert_eq!(result.is_ok(), true);
        let rendered = result.unwrap();
        assert_eq!(rendered.contains("foo_readme"), true);
        assert_eq!(rendered.contains("script"), false);
        assert_eq!(rendered.contains("alert('Hello World')"), true);
    }

    #[test]
    fn text_with_iframe_tag() {
        let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
        let result = markdown_to_html(text);
        assert_eq!(result.is_ok(), true);
        let rendered = result.unwrap();
        assert_eq!(rendered.contains("foo_readme"), true);
        assert_eq!(rendered.contains("iframe"), false);
        assert_eq!(rendered.contains("alert('Hello World')"), true);
    }

    #[test]
    fn text_with_unknwon_tag() {
        let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
        let result = markdown_to_html(text);
        assert_eq!(result.is_ok(), true);
        let rendered = result.unwrap();
        assert_eq!(rendered.contains("foo_readme"), true);
        assert_eq!(rendered.contains("unknown"), false);
        assert_eq!(rendered.contains("alert('Hello World')"), true);
    }

    #[test]
    fn text_with_inline_javascript() {
        let text = r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
        let result = markdown_to_html(text);
        assert_eq!(result.is_ok(), true);
        let rendered = result.unwrap();
        assert_eq!(rendered.contains("foo_readme"), true);
        assert_eq!(rendered.contains("<a"), true);
        assert_eq!(rendered.contains("href="), true);
        assert_eq!(rendered.contains("onclick"), false);
        assert_eq!(rendered.contains("window.alert"), false);
        assert_eq!(rendered.contains("Crate page"), true);
    }
}

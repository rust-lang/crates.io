use ammonia::Ammonia;
use pulldown_cmark::Parser;
use pulldown_cmark::html;

use util::CargoResult;

/// Context for markdown to HTML rendering.
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
            "i",
            "h1",
            "h2",
            "h3",
            "hr",
            "img",
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
            ("pre", ["style"].iter().cloned().collect()),
            ("span", ["style"].iter().cloned().collect()),
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
        let mut rendered = String::with_capacity(text.len() * 3 / 2);
        let parser = Parser::new(text);
        html::push_html(&mut rendered, parser);
        Ok(self.html_sanitizer.clean(&rendered))
    }
}

impl<'a> Default for MarkdownRenderer<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders a markdown text to sanitized HTML. The returned text should not contain any harmful
/// HTML tag or attribute (such as iframe, onclick, onmouseover, etc.).
pub fn markdown_to_html(text: &str) -> CargoResult<String> {
    let renderer = MarkdownRenderer::new();
    renderer.to_html(text)
}

use ammonia::{Ammonia, UrlRelative};
use comrak;

use util::CargoResult;

/// Context for markdown to HTML rendering.
#[allow(missing_debug_implementations)]
pub struct MarkdownRenderer<'a> {
    html_sanitizer: Ammonia<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates a new renderer instance.
    pub fn new(repo: Option<&'a str>) -> MarkdownRenderer<'a> {
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
            ("code", ["class"].iter().cloned().collect()),
            (
                "img",
                ["width", "height", "src", "alt", "align"]
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
        let allowed_classes = [
            (
                "code",
                [
                    "language-bash",
                    "language-clike",
                    "language-glsl",
                    "language-go",
                    "language-ini",
                    "language-javascript",
                    "language-json",
                    "language-markup",
                    "language-protobuf",
                    "language-ruby",
                    "language-rust",
                    "language-scss",
                    "language-sql",
                    "yaml",
                ].iter()
                    .cloned()
                    .collect(),
            ),
        ].iter()
            .cloned()
            .collect();
        let url_relative = if let Some(repo) = repo {
            UrlRelative::RewriteWithBase(repo)
        } else {
            UrlRelative::Deny
        };
        let html_sanitizer = Ammonia {
            link_rel: Some("nofollow noopener noreferrer"),
            keep_cleaned_elements: true,
            tags: tags,
            tag_attributes: tag_attributes,
            allowed_classes: allowed_classes,
            url_relative: url_relative,
            ..Ammonia::default()
        };
        MarkdownRenderer {
            html_sanitizer: html_sanitizer,
        }
    }

    /// Renders the given markdown to HTML using the current settings.
    pub fn to_html(&self, text: &str) -> CargoResult<String> {
        let options = comrak::ComrakOptions {
            ext_autolink: true,
            ext_strikethrough: true,
            ext_table: true,
            ext_tagfilter: true,
            ext_tasklist: true,
            ..comrak::ComrakOptions::default()
        };
        let rendered = comrak::markdown_to_html(text, &options);
        Ok(self.html_sanitizer.clean(&rendered))
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
/// let rendered = markdown_to_html(text, None)?;
/// ```
pub fn markdown_to_html(text: &str, repo: Option<&str>) -> CargoResult<String> {
    let repo = repo.map(|r|
        format!("{}{}blob/master/", r, if r.ends_with("/") { "" } else { "/" }));

    let renderer = MarkdownRenderer::new(repo.as_ref().map(|s| &**s));
    renderer.to_html(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text() {
        let text = "";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn text_with_script_tag() {
        let text = "foo_readme\n\n<script>alert('Hello World')</script>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;script&gt;alert(\'Hello World\')&lt;/script&gt;\n"
        );
    }

    #[test]
    fn text_with_iframe_tag() {
        let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;iframe&gt;alert(\'Hello World\')&lt;/iframe&gt;\n"
        );
    }

    #[test]
    fn text_with_unknown_tag() {
        let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "<p>foo_readme</p>\n<p>alert(\'Hello World\')</p>\n");
    }

    #[test]
    fn text_with_inline_javascript() {
        let text = r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<p>foo_readme\\n\\n<a href=\"https://crates.io/crates/cargo-registry\" rel=\"nofollow noopener noreferrer\">Crate page</a></p>\n"
        );
    }

    // See https://github.com/kivikakk/comrak/issues/37. This panic happened
    // in comrak 0.1.8 but was fixed in 0.1.9.
    #[test]
    fn text_with_fancy_single_quotes() {
        let text = r#"wb’"#;
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "<p>wb’</p>\n");
    }

    #[test]
    fn code_block_with_syntax_highlighting() {
        let code_block = r#"```rust \
                            println!("Hello World"); \
                           ```"#;
        let result = markdown_to_html(code_block, None).unwrap();
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn text_with_forbidden_class_attribute() {
        let text = "<p class='bad-class'>Hello World!</p>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "<p>Hello World!</p>\n");
    }

    #[test]
    fn relative_links() {
        // The commented out behaviour is desirable, but not possible
        // with what ammonia currently offers.

        // let absolute = "[hi](/hi)";
        let relative = "[there](there)";

        for url in &["https://github.com/rust-lang/test",
                     "https://github.com/rust-lang/test/"] {
            // let result = markdown_to_html(absolute, Some(url)).unwrap();
            // assert_eq!(result, "<p><a href=\"https://github.com/rust-lang/test/blob/master/hi\" rel=\"nofollow noopener noreferrer\">hi</a>");

            let result = markdown_to_html(relative, Some(url)).unwrap();
            assert_eq!(result, "<p><a href=\"https://github.com/rust-lang/test/blob/master/there\" rel=\"nofollow noopener noreferrer\">there</a></p>\n");
        }
    }
}

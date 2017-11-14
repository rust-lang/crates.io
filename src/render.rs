use ammonia::{Builder, UrlRelative};
use comrak;
use htmlescape::encode_minimal;
use std::borrow::Cow;
use url::Url;

use util::CargoResult;

/// Context for markdown to HTML rendering.
#[allow(missing_debug_implementations)]
struct MarkdownRenderer<'a> {
    html_sanitizer: Builder<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates a new renderer instance.
    ///
    /// Per `readme_to_html`, `base_url` is the base URL prepended to any
    /// relative links in the input document.  See that function for more detail.
    fn new(base_url: Option<&'a str>) -> MarkdownRenderer<'a> {
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
            ("a", ["href", "id", "target"].iter().cloned().collect()),
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

        let sanitizer_base_url = base_url.map(|s| s.to_string());

        // Constrain the type of the closures given to the HTML sanitizer.
        fn constrain_closure<F>(f: F) -> F
        where
            F: for<'a> Fn(&'a str) -> Option<Cow<'a, str>> + Send + Sync,
        {
            f
        }

        let unrelative_url_sanitizer = constrain_closure(|url| {
            // We have no base URL; allow fragment links only.
            if url.starts_with('#') {
                return Some(Cow::Borrowed(url));
            }

            None
        });

        let relative_url_sanitizer = constrain_closure(move |url| {
            // sanitizer_base_url is Some(String); use it to fix the relative URL.
            if url.starts_with('#') {
                return Some(Cow::Borrowed(url));
            }

            let mut new_url = sanitizer_base_url.clone().unwrap();
            if !new_url.ends_with('/') {
                new_url.push('/');
            }
            if new_url.ends_with(".git/") {
                let offset = new_url.len() - 5;
                new_url.drain(offset..offset + 4);
            }
            new_url += "blob/master";
            if !url.starts_with('/') {
                new_url.push('/');
            }
            new_url += url;
            Some(Cow::Owned(new_url))
        });

        let use_relative = if let Some(base_url) = base_url {
            if let Ok(url) = Url::parse(base_url) {
                url.host_str() == Some("github.com") || url.host_str() == Some("gitlab.com")
                    || url.host_str() == Some("bitbucket.org")
            } else {
                false
            }
        } else {
            false
        };

        let mut html_sanitizer = Builder::new();
        html_sanitizer
            .link_rel(Some("nofollow noopener noreferrer"))
            .tags(tags)
            .tag_attributes(tag_attributes)
            .allowed_classes(allowed_classes)
            .url_relative(if use_relative {
                UrlRelative::Custom(Box::new(relative_url_sanitizer))
            } else {
                UrlRelative::Custom(Box::new(unrelative_url_sanitizer))
            })
            .id_prefix(Some("user-content-"));

        MarkdownRenderer {
            html_sanitizer: html_sanitizer,
        }
    }

    /// Renders the given markdown to HTML using the current settings.
    fn to_html(&self, text: &str) -> CargoResult<String> {
        let options = comrak::ComrakOptions {
            ext_autolink: true,
            ext_strikethrough: true,
            ext_table: true,
            ext_tagfilter: true,
            ext_tasklist: true,
            ext_header_ids: Some("user-content-".to_string()),
            ..comrak::ComrakOptions::default()
        };
        let rendered = comrak::markdown_to_html(text, &options);
        Ok(self.html_sanitizer.clean(&rendered).to_string())
    }
}

/// Renders Markdown text to sanitized HTML with a given `base_url`.
/// See `readme_to_html` for the interpretation of `base_url`.
fn markdown_to_html(text: &str, base_url: Option<&str>) -> CargoResult<String> {
    let renderer = MarkdownRenderer::new(base_url);
    renderer.to_html(text)
}

/// Any readme with a filename ending in one of these extensions will be rendered as Markdown.
/// Note we also render a readme as Markdown if _no_ extension is on the filename.
static MARKDOWN_EXTENSIONS: [&'static str; 7] = [
    ".md",
    ".markdown",
    ".mdown",
    ".mdwn",
    ".mkd",
    ".mkdn",
    ".mkdown",
];

/// Renders a readme to sanitized HTML.  An appropriate rendering method is chosen depending
/// on the extension of the supplied `filename`.
///
/// The returned text will not contain any harmful HTML tag or attribute (such as iframe,
/// onclick, onmouseover, etc.).
///
/// The `base_url` parameter will be used as the base for any relative links found in the
/// Markdown, as long as its host part is github.com, gitlab.com, or bitbucket.org.  The
/// supplied URL will be used as a directory base whether or not the relative link is
/// prefixed with '/'.  If `None` is passed, relative links will be omitted.
///
/// # Examples
///
/// ```
/// use render::render_to_html;
///
/// let text = "[Rust](https://rust-lang.org/) is an awesome *systems programming* language!";
/// let rendered = readme_to_html(text, "README.md", None)?;
/// ```
pub fn readme_to_html(text: &str, filename: &str, base_url: Option<&str>) -> CargoResult<String> {
    let filename = filename.to_lowercase();

    if !filename.contains('.') || MARKDOWN_EXTENSIONS.iter().any(|e| filename.ends_with(e)) {
        return markdown_to_html(text, base_url);
    }

    Ok(encode_minimal(text).replace("\n", "<br>\n"))
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
        let text =
            r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
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
        let absolute = "[hi](/hi)";
        let relative = "[there](there)";

        for host in &["github.com", "gitlab.com", "bitbucket.org"] {
            for (&extra_slash, &dot_git) in [true, false].iter().zip(&[true, false]) {
                let url = format!(
                    "https://{}/rust-lang/test{}{}",
                    host,
                    if dot_git { ".git" } else { "" },
                    if extra_slash { "/" } else { "" },
                );

                let result = markdown_to_html(absolute, Some(&url)).unwrap();
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{}/rust-lang/test/blob/master/hi\" rel=\"nofollow noopener noreferrer\">hi</a></p>\n",
                        host
                    )
                );

                let result = markdown_to_html(relative, Some(&url)).unwrap();
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{}/rust-lang/test/blob/master/there\" rel=\"nofollow noopener noreferrer\">there</a></p>\n",
                        host
                    )
                );
            }
        }

        let result = markdown_to_html(absolute, Some("https://google.com/")).unwrap();
        assert_eq!(
            result,
            "<p><a rel=\"nofollow noopener noreferrer\">hi</a></p>\n"
        );
    }

    #[test]
    fn absolute_links_dont_get_resolved() {
        let readme_text =
            "[![Crates.io](https://img.shields.io/crates/v/clap.svg)](https://crates.io/crates/clap)";
        let repository = "https://github.com/kbknapp/clap-rs/";
        let result = markdown_to_html(readme_text, Some(&repository)).unwrap();

        assert_eq!(
            result,
            "<p><a href=\"https://crates.io/crates/clap\" rel=\"nofollow noopener noreferrer\"><img src=\"https://img.shields.io/crates/v/clap.svg\" alt=\"Crates.io\"></a></p>\n"
        );
    }

    #[test]
    fn readme_to_html_renders_markdown() {
        for f in &["README", "readme.md", "README.MARKDOWN", "whatever.mkd"] {
            assert_eq!(
                readme_to_html("*lobster*", f, None).unwrap(),
                "<p><em>lobster</em></p>\n"
            );
        }
    }

    #[test]
    fn readme_to_html_renders_other_things() {
        for f in &["readme.exe", "readem.org", "blah.adoc"] {
            assert_eq!(
                readme_to_html("<script>lobster</script>\n\nis my friend\n", f, None).unwrap(),
                "&lt;script&gt;lobster&lt;/script&gt;<br>\n<br>\nis my friend<br>\n"
            );
        }
    }

    #[test]
    fn header_has_tags() {
        let text = "# My crate\n\nHello, world!\n";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<h1><a href=\"#my-crate\" id=\"user-content-my-crate\" rel=\"nofollow noopener noreferrer\"></a>My crate</h1>\n<p>Hello, world!</p>\n"
        );
    }

    #[test]
    fn manual_anchor_is_sanitized() {
        let text =
            "<h1><a href=\"#my-crate\" id=\"my-crate\"></a>My crate</h1>\n<p>Hello, world!</p>\n";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<h1><a href=\"#my-crate\" id=\"user-content-my-crate\" rel=\"nofollow noopener noreferrer\"></a>My crate</h1>\n<p>Hello, world!</p>\n"
        );
    }
}

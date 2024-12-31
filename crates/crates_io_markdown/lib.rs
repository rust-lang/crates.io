#![doc = include_str!("README.md")]

use ammonia::{Builder, UrlRelative, UrlRelativeEvaluate};
use comrak::nodes::{AstNode, NodeValue};
use htmlescape::encode_minimal;
use std::borrow::Cow;
use std::path::Path;
use url::Url;

/// Context for markdown to HTML rendering.
struct MarkdownRenderer<'a> {
    html_sanitizer: Builder<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates a new renderer instance.
    ///
    /// Per `text_to_html`, `base_url` is the base URL prepended to any
    /// relative links in the input document.  See that function for more detail.
    fn new(base_url: Option<&'a str>, base_dir: &'a str) -> MarkdownRenderer<'a> {
        let allowed_classes = hashmap(&[
            (
                "code",
                hashset(&[
                    // Languages
                    "language-bash",
                    "language-c",
                    "language-glsl",
                    "language-go",
                    "language-ini",
                    "language-javascript",
                    "language-json",
                    "language-xml",
                    "language-mermaid",
                    "language-protobuf",
                    "language-ruby",
                    "language-rust",
                    "language-scss",
                    "language-sql",
                    "language-toml",
                    "language-yaml",
                    // Aliases
                    "language-rs",
                    "language-clike",
                    "language-markup",
                ]),
            ),
            ("section", hashset(&["footnotes"])),
        ]);
        let sanitize_url = UrlRelative::Custom(Box::new(SanitizeUrl::new(base_url, base_dir)));

        let mut html_sanitizer = Builder::default();
        html_sanitizer
            .add_tags(&["input", "ol", "picture", "section", "source"])
            .link_rel(Some("nofollow noopener noreferrer"))
            .add_generic_attributes(&["align"])
            .add_tag_attributes("a", &["id", "target"])
            .add_tag_attributes("input", &["checked", "disabled", "type"])
            .add_tag_attributes("li", &["id"])
            .add_tag_attributes("source", &["media", "srcset"])
            .allowed_classes(allowed_classes)
            .url_relative(sanitize_url)
            .id_prefix(Some("user-content-"));
        MarkdownRenderer { html_sanitizer }
    }

    /// Renders the given markdown to HTML using the current settings.
    fn to_html(&self, text: &str) -> String {
        use comrak::{
            format_html, parse_document, Arena, ComrakExtensionOptions, ComrakOptions,
            ComrakRenderOptions,
        };

        let render_options = ComrakRenderOptions::builder()
            // The output will be sanitized with `ammonia`
            .unsafe_(true)
            .build();

        let extension_options = ComrakExtensionOptions::builder()
            .autolink(true)
            .strikethrough(true)
            .table(true)
            .tagfilter(true)
            .tasklist(true)
            .header_ids("user-content-".to_string())
            .footnotes(true)
            .build();

        let options = ComrakOptions {
            render: render_options,
            extension: extension_options,
            ..ComrakOptions::default()
        };

        let arena = Arena::new();
        let root = parse_document(&arena, text, &options);

        // Tweak annotations of code blocks.
        iter_nodes(root, &|node| {
            if let NodeValue::CodeBlock(ref mut ncb) = node.data.borrow_mut().value {
                let orig_annot = ncb.info.as_str();

                // Ignore characters after a comma for syntax highlighting to work correctly.
                if let Some((before_comma, _)) = orig_annot.split_once(',') {
                    ncb.info = before_comma.to_string();
                }
            }
        });

        let mut html = Vec::new();
        format_html(root, &options, &mut html).unwrap();
        let rendered = String::from_utf8(html).unwrap();
        self.html_sanitizer.clean(&rendered).to_string()
    }
}

/// Iterate the nodes in the CommonMark AST, used in comrak.
fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
where
    F: Fn(&'a AstNode<'a>),
{
    f(node);
    for c in node.children() {
        iter_nodes(c, f);
    }
}

/// Add trailing slash and remove `.git` suffix of base URL.
fn canon_base_url(mut base_url: String) -> String {
    if !base_url.ends_with('/') {
        base_url.push('/');
    }
    if base_url.ends_with(".git/") {
        let offset = base_url.len() - 5;
        base_url.drain(offset..offset + 4);
    }
    base_url
}

/// Sanitize relative URLs in Markdown files.
struct SanitizeUrl {
    base_url: Option<String>,
    base_dir: String,
}

impl SanitizeUrl {
    fn new(base_url: Option<&str>, base_dir: &str) -> Self {
        let base_url = base_url
            .and_then(|base_url| Url::parse(base_url).ok())
            .and_then(|url| match url.host_str() {
                Some("github.com") | Some("gitlab.com") | Some("bitbucket.org") => {
                    Some(canon_base_url(url.into()))
                }
                _ => None,
            });
        Self {
            base_url,
            base_dir: base_dir.to_owned(),
        }
    }
}

/// Groups media-related URL info
struct MediaUrl {
    is_media: bool,
    add_sanitize_query: bool,
}

/// Determine whether the given URL has a media file extension.
/// Also check if `sanitize=true` must be added to the query string,
/// which is required to load SVGs properly from GitHub.
fn is_media_url(url: &str) -> MediaUrl {
    Path::new(url)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map_or(
            MediaUrl {
                is_media: false,
                add_sanitize_query: false,
            },
            |e| match e {
                "svg" => MediaUrl {
                    is_media: true,
                    add_sanitize_query: true,
                },
                "png" | "jpg" | "jpeg" | "gif" | "mp4" | "webm" | "ogg" | "webp" => MediaUrl {
                    is_media: true,
                    add_sanitize_query: false,
                },
                _ => MediaUrl {
                    is_media: false,
                    add_sanitize_query: false,
                },
            },
        )
}

impl UrlRelativeEvaluate<'_> for SanitizeUrl {
    fn evaluate<'a>(&self, url: &'a str) -> Option<Cow<'a, str>> {
        if let Some(clean) = url.strip_prefix('#') {
            // Handle auto-generated footnote links
            if clean.starts_with("fn-") || clean.starts_with("fnref-") {
                return Some(Cow::Owned(format!("#user-content-{}", clean)));
            }

            // Always allow fragment URLs.
            return Some(Cow::Borrowed(url));
        }

        if url.starts_with("::") {
            // Always reject relative rustdoc URLs.
            return None;
        }

        self.base_url.as_ref().map(|base_url| {
            let mut new_url = base_url.clone();
            // Assumes GitHub’s URL scheme. GitHub renders text and markdown
            // better in the "blob" view, but images need to be served raw.
            let MediaUrl {
                is_media,
                add_sanitize_query,
            } = is_media_url(url);
            new_url += if is_media { "raw/HEAD" } else { "blob/HEAD" };
            if !self.base_dir.is_empty() {
                new_url += "/";
                new_url += &self.base_dir;
            }
            if !url.starts_with('/') {
                new_url.push('/');
            }
            new_url += url;
            if add_sanitize_query {
                if let Ok(mut parsed_url) = Url::parse(&new_url) {
                    parsed_url.query_pairs_mut().append_pair("sanitize", "true");
                    new_url = parsed_url.into();
                }
            }
            Cow::Owned(new_url)
        })
    }
}

/// Renders Markdown text to sanitized HTML with a given `base_url`.
/// See `text_to_html` for the interpretation of `base_url`.
fn markdown_to_html(text: &str, base_url: Option<&str>, base_dir: &str) -> String {
    let renderer = MarkdownRenderer::new(base_url, base_dir);
    renderer.to_html(text)
}

/// Any file with a filename ending in one of these extensions will be rendered as Markdown.
/// Note we also render a file as Markdown if _no_ extension is on the filename.
static MARKDOWN_EXTENSIONS: [&str; 7] =
    ["md", "markdown", "mdown", "mdwn", "mkd", "mkdn", "mkdown"];

/// Renders a text file to sanitized HTML.  An appropriate rendering method is chosen depending
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
/// use crates_io_markdown::text_to_html;
///
/// let text = "[Rust](https://rust-lang.org/) is an awesome *systems programming* language!";
/// let rendered = text_to_html(text, "README.md", None, None);
/// assert_eq!(rendered, "<p><a href=\"https://rust-lang.org/\" rel=\"nofollow noopener noreferrer\">Rust</a> is an awesome <em>systems programming</em> language!</p>\n");
/// ```
pub fn text_to_html<P: AsRef<Path>>(
    text: &str,
    readme_path_in_pkg: P,
    base_url: Option<&str>,
    pkg_path_in_vcs: Option<P>,
) -> String {
    let path_in_vcs = match pkg_path_in_vcs {
        None => readme_path_in_pkg.as_ref().to_path_buf(),
        Some(pkg_path_in_vcs) => pkg_path_in_vcs.as_ref().join(readme_path_in_pkg),
    };

    let base_dir = path_in_vcs.parent().and_then(|p| p.to_str()).unwrap_or("");

    if path_in_vcs.extension().is_none() {
        return markdown_to_html(text, base_url, base_dir);
    }

    if let Some(ext) = path_in_vcs.extension().and_then(|ext| ext.to_str()) {
        if MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return markdown_to_html(text, base_url, base_dir);
        }
    }

    encode_minimal(text).replace('\n', "<br>\n")
}

/// Helper function to build a new `HashSet` from the items slice.
fn hashset<T>(items: &[T]) -> std::collections::HashSet<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    items.iter().cloned().collect()
}

/// Helper function to build a new `HashMap` from a slice of key-value pairs.
fn hashmap<K, V>(items: &[(K, V)]) -> std::collections::HashMap<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    items.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn empty_text() {
        let text = "";
        assert_eq!(markdown_to_html(text, None, ""), "");
    }

    #[test]
    fn text_with_script_tag() {
        let text = "foo_readme\n\n<script>alert('Hello World')</script>";
        assert_snapshot!(markdown_to_html(text, None, ""), @r"
        <p>foo_readme</p>
        &lt;script&gt;alert('Hello World')&lt;/script&gt;
        ");
    }

    #[test]
    fn text_with_iframe_tag() {
        let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
        assert_snapshot!(markdown_to_html(text, None, ""), @r"
        <p>foo_readme</p>
        &lt;iframe&gt;alert('Hello World')&lt;/iframe&gt;
        ");
    }

    #[test]
    fn text_with_unknown_tag() {
        let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
        assert_snapshot!(markdown_to_html(text, None, ""), @r"
        <p>foo_readme</p>
        <p>alert('Hello World')</p>
        ");
    }

    #[test]
    fn text_with_kbd_tag() {
        let text = "foo_readme\n\nHello <kbd>alert('Hello World')</kbd>";
        assert_snapshot!(markdown_to_html(text, None, ""), @r"
        <p>foo_readme</p>
        <p>Hello <kbd>alert('Hello World')</kbd></p>
        ");
    }

    #[test]
    fn text_with_inline_javascript() {
        let text = r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
        assert_snapshot!(markdown_to_html(text, None, ""), @r#"<p>foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" rel="nofollow noopener noreferrer">Crate page</a></p>"#);
    }

    // See https://github.com/kivikakk/comrak/issues/37. This panic happened
    // in comrak 0.1.8 but was fixed in 0.1.9.
    #[test]
    fn text_with_fancy_single_quotes() {
        let text = "wb’";
        assert_snapshot!(markdown_to_html(text, None, ""), @"<p>wb’</p>");
    }

    #[test]
    fn code_block_with_syntax_highlighting() {
        let code_block = "```rust\nprintln!(\"Hello World\");\n```";
        assert_snapshot!(markdown_to_html(code_block, None, ""), @r#"
        <pre><code class="language-rust">println!("Hello World");
        </code></pre>
        "#);
    }

    #[test]
    fn code_block_with_mermaid_highlighting() {
        let code_block = "```mermaid\ngraph LR\nA --> C\nC --> A\n```";
        assert_snapshot!(markdown_to_html(code_block, None, ""), @r#"
        <pre><code class="language-mermaid">graph LR
        A --&gt; C
        C --&gt; A
        </code></pre>
        "#);
    }

    #[test]
    fn code_block_with_syntax_highlighting_even_if_annot_has_no_run() {
        let code_block = "```rust, no_run\nprintln!(\"Hello World\");\n```";
        assert_snapshot!(markdown_to_html(code_block, None, ""), @r#"
        <pre><code class="language-rust">println!("Hello World");
        </code></pre>
        "#);
    }

    #[test]
    fn code_block_with_syntax_highlighting_with_aliases() {
        let code_block = "```rs, no_run\nprintln!(\"Hello World\");\n```";
        assert_snapshot!(markdown_to_html(code_block, None, ""), @r#"
        <pre><code class="language-rs">println!("Hello World");
        </code></pre>
        "#);

        let code_block = "```markup, no_run\n<hello>World</hello>\n```";
        assert_snapshot!(markdown_to_html(code_block, None, ""), @r#"
        <pre><code class="language-markup">&lt;hello&gt;World&lt;/hello&gt;
        </code></pre>
        "#);

        let code_block = "```clike, no_run\nint main() { }\n```";
        assert_snapshot!(markdown_to_html(code_block, None, ""), @r#"
        <pre><code class="language-clike">int main() { }
        </code></pre>
        "#);
    }

    #[test]
    fn text_with_forbidden_class_attribute() {
        let text = "<p class='bad-class'>Hello World!</p>";
        assert_snapshot!(markdown_to_html(text, None, ""), @"<p>Hello World!</p>");
    }

    #[test]
    fn text_with_footnote() {
        let text = "Hello World![^1]\n\n[^1]: Hello Ferris, actually!";
        assert_snapshot!(markdown_to_html(text, None, ""), @r##"
        <p>Hello World!<sup><a href="#user-content-fn-1" id="user-content-fnref-1" rel="nofollow noopener noreferrer">1</a></sup></p>
        <section class="footnotes">
        <ol>
        <li id="user-content-fn-1">
        <p>Hello Ferris, actually! <a href="#user-content-fnref-1" rel="nofollow noopener noreferrer">↩</a></p>
        </li>
        </ol>
        </section>
        "##);
    }

    #[test]
    fn text_with_complex_footnotes() {
        let text = r#"Here's a simple footnote,[^1] and here's a longer one.[^bignote]

[^1]: This is the first footnote.

There can also be some text in between!

[^bignote]: Here's one with multiple paragraphs and code.

    Indent paragraphs to include them in the footnote.

    `{ my code }`

    Add as many paragraphs as you like."#;

        assert_snapshot!(markdown_to_html(text, None, ""), @r##"
        <p>Here's a simple footnote,<sup><a href="#user-content-fn-1" id="user-content-fnref-1" rel="nofollow noopener noreferrer">1</a></sup> and here's a longer one.<sup><a href="#user-content-fn-bignote" id="user-content-fnref-bignote" rel="nofollow noopener noreferrer">2</a></sup></p>
        <p>There can also be some text in between!</p>
        <section class="footnotes">
        <ol>
        <li id="user-content-fn-1">
        <p>This is the first footnote. <a href="#user-content-fnref-1" rel="nofollow noopener noreferrer">↩</a></p>
        </li>
        <li id="user-content-fn-bignote">
        <p>Here's one with multiple paragraphs and code.</p>
        <p>Indent paragraphs to include them in the footnote.</p>
        <p><code>{ my code }</code></p>
        <p>Add as many paragraphs as you like. <a href="#user-content-fnref-bignote" rel="nofollow noopener noreferrer">↩</a></p>
        </li>
        </ol>
        </section>
        "##);
    }

    #[test]
    fn relative_links() {
        let absolute = "[hi](/hi)";
        let relative = "[there](there)";
        let image = "![alt](img.png)";
        let html_image = "<img src=\"img.png\" alt=\"alt\">";
        let svg = "![alt](sanitize.svg)";

        for host in &["github.com", "gitlab.com", "bitbucket.org"] {
            for (&extra_slash, &dot_git) in [true, false].iter().zip(&[true, false]) {
                let url = format!(
                    "https://{}/rust-lang/test{}{}",
                    host,
                    if dot_git { ".git" } else { "" },
                    if extra_slash { "/" } else { "" },
                );

                let result = markdown_to_html(absolute, Some(&url), "");
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{host}/rust-lang/test/blob/HEAD/hi\" rel=\"nofollow noopener noreferrer\">hi</a></p>\n"
                    )
                );

                let result = markdown_to_html(relative, Some(&url), "");
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{host}/rust-lang/test/blob/HEAD/there\" rel=\"nofollow noopener noreferrer\">there</a></p>\n"
                    )
                );

                let result = markdown_to_html(image, Some(&url), "");
                assert_eq!(
                    result,
                    format!(
                        "<p><img src=\"https://{host}/rust-lang/test/raw/HEAD/img.png\" alt=\"alt\"></p>\n",
                    )
                );

                let result = markdown_to_html(html_image, Some(&url), "");
                assert_eq!(
                    result,
                    format!(
                        "<img src=\"https://{host}/rust-lang/test/raw/HEAD/img.png\" alt=\"alt\">\n",
                    )
                );

                let result = markdown_to_html(svg, Some(&url), "");
                assert_eq!(
                    result,
                    format!(
                        "<p><img src=\"https://{host}/rust-lang/test/raw/HEAD/sanitize.svg?sanitize=true\" alt=\"alt\"></p>\n",
                    )
                );

                let result = markdown_to_html(svg, Some(&url), "subdir");
                assert_eq!(
                    result,
                    format!(
                        "<p><img src=\"https://{host}/rust-lang/test/raw/HEAD/subdir/sanitize.svg?sanitize=true\" alt=\"alt\"></p>\n",
                    )
                );

                let result = markdown_to_html(svg, Some(&url), "subdir1/subdir2");
                assert_eq!(
                    result,
                    format!(
                        "<p><img src=\"https://{host}/rust-lang/test/raw/HEAD/subdir1/subdir2/sanitize.svg?sanitize=true\" alt=\"alt\"></p>\n",
                    )
                );
            }
        }

        let result = markdown_to_html(absolute, Some("https://google.com/"), "");
        assert_eq!(
            result,
            "<p><a rel=\"nofollow noopener noreferrer\">hi</a></p>\n"
        );
    }

    #[test]
    fn absolute_links_dont_get_resolved() {
        let text =
            "[![crates.io](https://img.shields.io/crates/v/clap.svg)](https://crates.io/crates/clap)";
        let repository = "https://github.com/kbknapp/clap-rs/";
        assert_snapshot!(markdown_to_html(text, Some(repository), ""), @r#"<p><a href="https://crates.io/crates/clap" rel="nofollow noopener noreferrer"><img src="https://img.shields.io/crates/v/clap.svg" alt="crates.io"></a></p>"#);
    }

    #[test]
    fn rustdoc_links() {
        let repository = "https://github.com/foo/bar/";

        assert_snapshot!(markdown_to_html("[stylish](::stylish)", Some(repository), ""), @r#"<p><a rel="nofollow noopener noreferrer">stylish</a></p>"#);

        assert_snapshot!(markdown_to_html("[Display](stylish::Display)", Some(repository), ""), @r#"<p><a rel="nofollow noopener noreferrer">Display</a></p>"#);
    }

    #[test]
    fn text_to_html_renders_markdown() {
        for f in &[
            "README",
            "readme.md",
            "README.MARKDOWN",
            "whatever.mkd",
            "s/readme.md",
            "s1/s2/readme.md",
        ] {
            assert_eq!(
                text_to_html("*lobster*", f, None, None),
                "<p><em>lobster</em></p>\n"
            );
        }

        assert_snapshot!(text_to_html("*[lobster](docs/lobster)*", "readme.md", Some("https://github.com/rust-lang/test"), None), @r#"<p><em><a href="https://github.com/rust-lang/test/blob/HEAD/docs/lobster" rel="nofollow noopener noreferrer">lobster</a></em></p>"#);
        assert_snapshot!(text_to_html("*[lobster](docs/lobster)*", "s/readme.md", Some("https://github.com/rust-lang/test"), None), @r#"<p><em><a href="https://github.com/rust-lang/test/blob/HEAD/s/docs/lobster" rel="nofollow noopener noreferrer">lobster</a></em></p>"#);
        assert_snapshot!(text_to_html("*[lobster](docs/lobster)*", "s1/s2/readme.md", Some("https://github.com/rust-lang/test"), None), @r#"<p><em><a href="https://github.com/rust-lang/test/blob/HEAD/s1/s2/docs/lobster" rel="nofollow noopener noreferrer">lobster</a></em></p>"#);
        assert_snapshot!(text_to_html("*[lobster](docs/lobster)*", "s1/s2/readme.md", Some("https://github.com/rust-lang/test"), Some("path/in/vcs/")), @r#"<p><em><a href="https://github.com/rust-lang/test/blob/HEAD/path/in/vcs/s1/s2/docs/lobster" rel="nofollow noopener noreferrer">lobster</a></em></p>"#);
        assert_snapshot!(text_to_html("*[lobster](docs/lobster)*", "s1/s2/readme.md", Some("https://github.com/rust-lang/test"), Some("path/in/vcs")), @r#"<p><em><a href="https://github.com/rust-lang/test/blob/HEAD/path/in/vcs/s1/s2/docs/lobster" rel="nofollow noopener noreferrer">lobster</a></em></p>"#);
    }

    #[test]
    fn text_to_html_renders_other_things() {
        for f in &["readme.exe", "readem.org", "blah.adoc"] {
            assert_eq!(
                text_to_html("<script>lobster</script>\n\nis my friend\n", f, None, None),
                "&lt;script&gt;lobster&lt;/script&gt;<br>\n<br>\nis my friend<br>\n"
            );
        }
    }

    #[test]
    fn header_has_tags() {
        let text = "# My crate\n\nHello, world!\n";
        assert_snapshot!(markdown_to_html(text, None, ""), @r##"
        <h1><a href="#my-crate" id="user-content-my-crate" rel="nofollow noopener noreferrer"></a>My crate</h1>
        <p>Hello, world!</p>
        "##);
    }

    #[test]
    fn manual_anchor_is_sanitized() {
        let text =
            "<h1><a href=\"#my-crate\" id=\"my-crate\"></a>My crate</h1>\n<p>Hello, world!</p>\n";
        assert_snapshot!(markdown_to_html(text, None, ""), @r##"
        <h1><a href="#my-crate" id="user-content-my-crate" rel="nofollow noopener noreferrer"></a>My crate</h1>
        <p>Hello, world!</p>
        "##);
    }

    #[test]
    fn tables_with_rowspan_and_colspan() {
        let text = "<table><tr><th rowspan=\"1\" colspan=\"2\">Target</th></tr></table>\n";
        assert_snapshot!(markdown_to_html(text, None, ""), @r#"<table><tbody><tr><th rowspan="1" colspan="2">Target</th></tr></tbody></table>"#);
    }

    #[test]
    fn text_alignment() {
        let text = "<h1 align=\"center\">foo-bar</h1>\n<h5 align=\"center\">Hello World!</h5>\n";
        assert_snapshot!(markdown_to_html(text, None, ""), @r#"
        <h1 align="center">foo-bar</h1>
        <h5 align="center">Hello World!</h5>
        "#);
    }

    #[test]
    fn image_alignment() {
        let text =
            "<p align=\"center\"><img src=\"https://img.shields.io/crates/v/clap.svg\" alt=\"\"></p>\n";
        assert_snapshot!(markdown_to_html(text, None, ""), @r#"<p align="center"><img src="https://img.shields.io/crates/v/clap.svg" alt=""></p>"#);
    }

    #[test]
    fn pictures_and_sources() {
        let text = r#"
<picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://test.crates.io/logo_dark.svg">
    <img src="https://test.crates.io/logo.svg" alt="logo" width="200">
</picture>
        "#;
        assert_snapshot!(markdown_to_html(text, None, ""), @r#"
        <picture>
            <source media="(prefers-color-scheme: dark)" srcset="https://test.crates.io/logo_dark.svg">
            <img src="https://test.crates.io/logo.svg" alt="logo" width="200">
        </picture>
        "#);
    }
}

//! Render Markdown files to HTML.

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
        let allowed_classes = hashmap(&[(
            "code",
            hashset(&[
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
                "language-toml",
                "language-yaml",
            ]),
        )]);
        let sanitize_url = UrlRelative::Custom(Box::new(SanitizeUrl::new(base_url, base_dir)));

        let mut html_sanitizer = Builder::default();
        html_sanitizer
            .add_tags(&["input"])
            .link_rel(Some("nofollow noopener noreferrer"))
            .add_generic_attributes(&["align"])
            .add_tag_attributes("a", &["id", "target"])
            .add_tag_attributes("input", &["checked", "disabled", "type"])
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

        let options = ComrakOptions {
            render: ComrakRenderOptions {
                unsafe_: true, // The output will be sanitized with `ammonia`
                ..ComrakRenderOptions::default()
            },
            extension: ComrakExtensionOptions {
                autolink: true,
                strikethrough: true,
                table: true,
                tagfilter: true,
                tasklist: true,
                header_ids: Some("user-content-".to_string()),
                ..ComrakExtensionOptions::default()
            },
            ..ComrakOptions::default()
        };

        let arena = Arena::new();
        let root = parse_document(&arena, text, &options);

        // Tweak annotations of code blocks.
        iter_nodes(root, &|node| {
            if let NodeValue::CodeBlock(ref mut ncb) = node.data.borrow_mut().value {
                // If annot includes invalid UTF-8 char, do nothing.
                if let Ok(mut orig_annot) = String::from_utf8(ncb.info.to_vec()) {
                    // Ignore characters after a comma for syntax highlighting to work correctly.
                    if let Some(offset) = orig_annot.find(',') {
                        let _ = orig_annot.drain(offset..orig_annot.len());
                        ncb.info = orig_annot.as_bytes().to_vec();
                    }
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

impl UrlRelativeEvaluate for SanitizeUrl {
    fn evaluate<'a>(&self, url: &'a str) -> Option<Cow<'a, str>> {
        if url.starts_with('#') {
            // Always allow fragment URLs.
            return Some(Cow::Borrowed(url));
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
/// use cargo_registry_markdown::text_to_html;
///
/// let text = "[Rust](https://rust-lang.org/) is an awesome *systems programming* language!";
/// let rendered = text_to_html(text, "README.md", None, None);
/// assert_eq!(rendered, "<p><a href=\"https://rust-lang.org/\" rel=\"nofollow noopener noreferrer\">Rust</a> is an awesome <em>systems programming</em> language!</p>\n");
/// ```
pub fn text_to_html(
    text: &str,
    readme_path_in_pkg: &str,
    base_url: Option<&str>,
    pkg_path_in_vcs: Option<&str>,
) -> String {
    let path_in_vcs = Path::new(pkg_path_in_vcs.unwrap_or("")).join(readme_path_in_pkg);
    let base_dir = path_in_vcs.parent().and_then(|p| p.to_str()).unwrap_or("");

    if path_in_vcs.extension().is_none() {
        return markdown_to_html(text, base_url, base_dir);
    }

    if let Some(ext) = path_in_vcs.extension().and_then(|ext| ext.to_str()) {
        if MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return markdown_to_html(text, base_url, base_dir);
        }
    }

    encode_minimal(text).replace("\n", "<br>\n")
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

    #[test]
    fn empty_text() {
        let text = "";
        let result = markdown_to_html(text, None, "");
        assert_eq!(result, "");
    }

    #[test]
    fn text_with_script_tag() {
        let text = "foo_readme\n\n<script>alert('Hello World')</script>";
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;script&gt;alert(\'Hello World\')&lt;/script&gt;\n"
        );
    }

    #[test]
    fn text_with_iframe_tag() {
        let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;iframe&gt;alert(\'Hello World\')&lt;/iframe&gt;\n"
        );
    }

    #[test]
    fn text_with_unknown_tag() {
        let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
        let result = markdown_to_html(text, None, "");
        assert_eq!(result, "<p>foo_readme</p>\n<p>alert(\'Hello World\')</p>\n");
    }

    #[test]
    fn text_with_inline_javascript() {
        let text = r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<p>foo_readme\\n\\n<a href=\"https://crates.io/crates/cargo-registry\" rel=\"nofollow noopener noreferrer\">Crate page</a></p>\n"
        );
    }

    // See https://github.com/kivikakk/comrak/issues/37. This panic happened
    // in comrak 0.1.8 but was fixed in 0.1.9.
    #[test]
    fn text_with_fancy_single_quotes() {
        let text = "wb’";
        let result = markdown_to_html(text, None, "");
        assert_eq!(result, "<p>wb’</p>\n");
    }

    #[test]
    fn code_block_with_syntax_highlighting() {
        let code_block = r#"```rust \
                            println!("Hello World"); \
                           ```"#;
        let result = markdown_to_html(code_block, None, "");
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn code_block_with_syntax_highlighting_even_if_annot_has_no_run() {
        let code_block = r#"```rust  ,  no_run \
                            println!("Hello World"); \
                           ```"#;
        let result = markdown_to_html(code_block, None, "");
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn text_with_forbidden_class_attribute() {
        let text = "<p class='bad-class'>Hello World!</p>";
        let result = markdown_to_html(text, None, "");
        assert_eq!(result, "<p>Hello World!</p>\n");
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
            "[![Crates.io](https://img.shields.io/crates/v/clap.svg)](https://crates.io/crates/clap)";
        let repository = "https://github.com/kbknapp/clap-rs/";
        let result = markdown_to_html(text, Some(repository), "");

        assert_eq!(
            result,
            "<p><a href=\"https://crates.io/crates/clap\" rel=\"nofollow noopener noreferrer\"><img src=\"https://img.shields.io/crates/v/clap.svg\" alt=\"Crates.io\"></a></p>\n"
        );
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

        assert_eq!(
            text_to_html("*[lobster](docs/lobster)*", "readme.md", Some("https://github.com/rust-lang/test"), None),
            "<p><em><a href=\"https://github.com/rust-lang/test/blob/HEAD/docs/lobster\" rel=\"nofollow noopener noreferrer\">lobster</a></em></p>\n"
        );
        assert_eq!(
            text_to_html("*[lobster](docs/lobster)*", "s/readme.md", Some("https://github.com/rust-lang/test"), None),
            "<p><em><a href=\"https://github.com/rust-lang/test/blob/HEAD/s/docs/lobster\" rel=\"nofollow noopener noreferrer\">lobster</a></em></p>\n"
        );
        assert_eq!(
            text_to_html("*[lobster](docs/lobster)*", "s1/s2/readme.md", Some("https://github.com/rust-lang/test"), None),
            "<p><em><a href=\"https://github.com/rust-lang/test/blob/HEAD/s1/s2/docs/lobster\" rel=\"nofollow noopener noreferrer\">lobster</a></em></p>\n"
        );
        assert_eq!(
            text_to_html("*[lobster](docs/lobster)*", "s1/s2/readme.md", Some("https://github.com/rust-lang/test"), Some("path/in/vcs/")),
            "<p><em><a href=\"https://github.com/rust-lang/test/blob/HEAD/path/in/vcs/s1/s2/docs/lobster\" rel=\"nofollow noopener noreferrer\">lobster</a></em></p>\n"
        );
        assert_eq!(
            text_to_html("*[lobster](docs/lobster)*", "s1/s2/readme.md", Some("https://github.com/rust-lang/test"), Some("path/in/vcs")),
            "<p><em><a href=\"https://github.com/rust-lang/test/blob/HEAD/path/in/vcs/s1/s2/docs/lobster\" rel=\"nofollow noopener noreferrer\">lobster</a></em></p>\n"
        );
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
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<h1><a href=\"#my-crate\" id=\"user-content-my-crate\" rel=\"nofollow noopener noreferrer\"></a>My crate</h1>\n<p>Hello, world!</p>\n"
        );
    }

    #[test]
    fn manual_anchor_is_sanitized() {
        let text =
            "<h1><a href=\"#my-crate\" id=\"my-crate\"></a>My crate</h1>\n<p>Hello, world!</p>\n";
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<h1><a href=\"#my-crate\" id=\"user-content-my-crate\" rel=\"nofollow noopener noreferrer\"></a>My crate</h1>\n<p>Hello, world!</p>\n"
        );
    }

    #[test]
    fn tables_with_rowspan_and_colspan() {
        let text = "<table><tr><th rowspan=\"1\" colspan=\"2\">Target</th></tr></table>\n";
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<table><tbody><tr><th rowspan=\"1\" colspan=\"2\">Target</th></tr></tbody></table>\n"
        );
    }

    #[test]
    fn text_alignment() {
        let text = "<h1 align=\"center\">foo-bar</h1>\n<h5 align=\"center\">Hello World!</h5>\n";
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<h1 align=\"center\">foo-bar</h1>\n<h5 align=\"center\">Hello World!</h5>\n"
        );
    }

    #[test]
    fn image_alignment() {
        let text =
            "<p align=\"center\"><img src=\"https://img.shields.io/crates/v/clap.svg\" alt=\"\"></p>\n";
        let result = markdown_to_html(text, None, "");
        assert_eq!(
            result,
            "<p align=\"center\"><img src=\"https://img.shields.io/crates/v/clap.svg\" alt=\"\"></p>\n"
        );
    }
}

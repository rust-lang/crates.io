//! Render README files to HTML.

use ammonia::{Builder, UrlRelative, UrlRelativeEvaluate};
use htmlescape::encode_minimal;
use std::borrow::Cow;
use std::path::Path;
use swirl::errors::PerformError;
use url::Url;

use crate::background_jobs::Environment;
use crate::models::Version;

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
        let tags = hashset(&[
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
            "h4",
            "h5",
            "h6",
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
        ]);
        let tag_attributes = hashmap(&[
            ("a", hashset(&["href", "id", "target"])),
            ("img", hashset(&["width", "height", "src", "alt", "align"])),
            ("input", hashset(&["checked", "disabled", "type"])),
        ]);
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
                "yaml",
            ]),
        )]);
        let sanitize_url = UrlRelative::Custom(Box::new(SanitizeUrl::new(base_url)));

        let mut html_sanitizer = Builder::new();
        html_sanitizer
            .link_rel(Some("nofollow noopener noreferrer"))
            .tags(tags)
            .tag_attributes(tag_attributes)
            .allowed_classes(allowed_classes)
            .url_relative(sanitize_url)
            .id_prefix(Some("user-content-"));
        MarkdownRenderer { html_sanitizer }
    }

    /// Renders the given markdown to HTML using the current settings.
    fn to_html(&self, text: &str) -> String {
        let options = comrak::ComrakOptions {
            unsafe_: true, // The output will be sanitized with `ammonia`
            ext_autolink: true,
            ext_strikethrough: true,
            ext_table: true,
            ext_tagfilter: true,
            ext_tasklist: true,
            ext_header_ids: Some("user-content-".to_string()),
            ..comrak::ComrakOptions::default()
        };
        let rendered = comrak::markdown_to_html(text, &options);
        self.html_sanitizer.clean(&rendered).to_string()
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

/// Sanitize relative URLs in README files.
struct SanitizeUrl {
    base_url: Option<String>,
}

impl SanitizeUrl {
    fn new(base_url: Option<&str>) -> Self {
        let base_url = base_url
            .and_then(|base_url| Url::parse(base_url).ok())
            .and_then(|url| match url.host_str() {
                Some("github.com") | Some("gitlab.com") | Some("bitbucket.org") => {
                    Some(canon_base_url(url.into_string()))
                }
                _ => None,
            });
        Self { base_url }
    }
}

/// Determine whether the given URL has a media file externsion.
fn is_media_url(url: &str) -> bool {
    Path::new(url)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map_or(false, |e| match e {
            "png" | "svg" | "jpg" | "jpeg" | "gif" | "mp4" | "webm" | "ogg" => true,
            _ => false,
        })
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
            new_url += if is_media_url(url) {
                "raw/master"
            } else {
                "blob/master"
            };
            if !url.starts_with('/') {
                new_url.push('/');
            }
            new_url += url;
            Cow::Owned(new_url)
        })
    }
}

/// Renders Markdown text to sanitized HTML with a given `base_url`.
/// See `readme_to_html` for the interpretation of `base_url`.
fn markdown_to_html(text: &str, base_url: Option<&str>) -> String {
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
pub fn readme_to_html(text: &str, filename: &str, base_url: Option<&str>) -> String {
    let filename = filename.to_lowercase();

    if !filename.contains('.') || MARKDOWN_EXTENSIONS.iter().any(|e| filename.ends_with(e)) {
        return markdown_to_html(text, base_url);
    }

    encode_minimal(text).replace("\n", "<br>\n")
}

#[swirl::background_job]
pub fn render_and_upload_readme(
    env: &Environment,
    version_id: i32,
    text: String,
    file_name: String,
    base_url: Option<String>,
) -> Result<(), PerformError> {
    use crate::schema::*;
    use crate::util::errors::std_error_no_send;
    use diesel::prelude::*;

    let rendered = readme_to_html(&text, &file_name, base_url.as_ref().map(String::as_str));
    let conn = env.connection()?;

    conn.transaction(|| {
        Version::record_readme_rendering(version_id, &conn)?;
        let (crate_name, vers) = versions::table
            .find(version_id)
            .inner_join(crates::table)
            .select((crates::name, versions::num))
            .first::<(String, String)>(&*conn)?;
        env.uploader
            .upload_readme(env.http_client(), &crate_name, &vers, rendered)
            .map_err(std_error_no_send)?;
        Ok(())
    })
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
        let result = markdown_to_html(text, None);
        assert_eq!(result, "");
    }

    #[test]
    fn text_with_script_tag() {
        let text = "foo_readme\n\n<script>alert('Hello World')</script>";
        let result = markdown_to_html(text, None);
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;script&gt;alert(\'Hello World\')&lt;/script&gt;\n"
        );
    }

    #[test]
    fn text_with_iframe_tag() {
        let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
        let result = markdown_to_html(text, None);
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;iframe&gt;alert(\'Hello World\')&lt;/iframe&gt;\n"
        );
    }

    #[test]
    fn text_with_unknown_tag() {
        let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
        let result = markdown_to_html(text, None);
        assert_eq!(result, "<p>foo_readme</p>\n<p>alert(\'Hello World\')</p>\n");
    }

    #[test]
    fn text_with_inline_javascript() {
        let text =
            r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
        let result = markdown_to_html(text, None);
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
        let result = markdown_to_html(text, None);
        assert_eq!(result, "<p>wb’</p>\n");
    }

    #[test]
    fn code_block_with_syntax_highlighting() {
        let code_block = r#"```rust \
                            println!("Hello World"); \
                           ```"#;
        let result = markdown_to_html(code_block, None);
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn text_with_forbidden_class_attribute() {
        let text = "<p class='bad-class'>Hello World!</p>";
        let result = markdown_to_html(text, None);
        assert_eq!(result, "<p>Hello World!</p>\n");
    }

    #[test]
    fn relative_links() {
        let absolute = "[hi](/hi)";
        let relative = "[there](there)";
        let image = "![alt](img.png)";

        for host in &["github.com", "gitlab.com", "bitbucket.org"] {
            for (&extra_slash, &dot_git) in [true, false].iter().zip(&[true, false]) {
                let url = format!(
                    "https://{}/rust-lang/test{}{}",
                    host,
                    if dot_git { ".git" } else { "" },
                    if extra_slash { "/" } else { "" },
                );

                let result = markdown_to_html(absolute, Some(&url));
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{}/rust-lang/test/blob/master/hi\" rel=\"nofollow noopener noreferrer\">hi</a></p>\n",
                        host
                    )
                );

                let result = markdown_to_html(relative, Some(&url));
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{}/rust-lang/test/blob/master/there\" rel=\"nofollow noopener noreferrer\">there</a></p>\n",
                        host
                    )
                );

                let result = markdown_to_html(image, Some(&url));
                assert_eq!(
                    result,
                    format!(
                 "<p><img src=\"https://{}/rust-lang/test/raw/master/img.png\" alt=\"alt\"></p>\n",
                        host
                    )
                );
            }
        }

        let result = markdown_to_html(absolute, Some("https://google.com/"));
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
        let result = markdown_to_html(readme_text, Some(repository));

        assert_eq!(
            result,
            "<p><a href=\"https://crates.io/crates/clap\" rel=\"nofollow noopener noreferrer\"><img src=\"https://img.shields.io/crates/v/clap.svg\" alt=\"Crates.io\"></a></p>\n"
        );
    }

    #[test]
    fn readme_to_html_renders_markdown() {
        for f in &["README", "readme.md", "README.MARKDOWN", "whatever.mkd"] {
            assert_eq!(
                readme_to_html("*lobster*", f, None),
                "<p><em>lobster</em></p>\n"
            );
        }
    }

    #[test]
    fn readme_to_html_renders_other_things() {
        for f in &["readme.exe", "readem.org", "blah.adoc"] {
            assert_eq!(
                readme_to_html("<script>lobster</script>\n\nis my friend\n", f, None),
                "&lt;script&gt;lobster&lt;/script&gt;<br>\n<br>\nis my friend<br>\n"
            );
        }
    }

    #[test]
    fn header_has_tags() {
        let text = "# My crate\n\nHello, world!\n";
        let result = markdown_to_html(text, None);
        assert_eq!(
            result,
            "<h1><a href=\"#my-crate\" id=\"user-content-my-crate\" rel=\"nofollow noopener noreferrer\"></a>My crate</h1>\n<p>Hello, world!</p>\n"
        );
    }

    #[test]
    fn manual_anchor_is_sanitized() {
        let text =
            "<h1><a href=\"#my-crate\" id=\"my-crate\"></a>My crate</h1>\n<p>Hello, world!</p>\n";
        let result = markdown_to_html(text, None);
        assert_eq!(
            result,
            "<h1><a href=\"#my-crate\" id=\"user-content-my-crate\" rel=\"nofollow noopener noreferrer\"></a>My crate</h1>\n<p>Hello, world!</p>\n"
        );
    }
}

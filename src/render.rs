use ammonia::Ammonia;
use pulldown_cmark::{Event, Parser, Tag};
use pulldown_cmark::html;
use std::borrow::Cow;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, ThemeSet};
use syntect::html::{IncludeBackground, styles_to_coloured_html};
use syntect::parsing::SyntaxSet;

use util::CargoResult;

/// Context for markdown to HTML rendering.
pub struct MarkdownRenderer<'a> {
    html_sanitizer: Ammonia<'a>,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
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
        MarkdownRenderer {
            html_sanitizer: html_sanitizer,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Renders the given markdown to HTML using the current settings.
    pub fn to_html(&self, text: &str) -> CargoResult<String> {
        let mut rendered = String::with_capacity(text.len() * 3 / 2);
        let mut codeblock = false;
        let mut sample = String::new();
        let parser = Parser::new(text).map(|event| match event.clone() {
            Event::Start(tag) => {
                match tag {
                    Tag::CodeBlock(_) => {
                        codeblock = true;
                        Event::Text(Cow::Borrowed(""))
                    }
                    _ => event,
                }
            }
            Event::End(tag) => {
                match tag {
                    Tag::CodeBlock(s) => {
                        let snippet = self.highlight(&s, &sample);
                        codeblock = false;
                        sample.clear();
                        Event::Html(Cow::Owned(snippet))
                    }
                    _ => event,
                }
            }
            Event::Text(t) => {
                if codeblock {
                    sample.push_str(&t);
                    Event::Text(Cow::Borrowed(""))
                } else {
                    event
                }
            }
            _ => event,
        });
        html::push_html(&mut rendered, parser);
        Ok(self.html_sanitizer.clean(&rendered))
    }

    /// Highlights to given code sample, using the syntax_hint parameter in order to choose the
    /// syntax it will use. Defaults to plain text.
    fn highlight(&self, syntax_hint: &str, sample: &str) -> String {
        use std::fmt::Write;

        let theme = &self.theme_set.themes["InspiredGitHub"];
        let syntax = self.syntax_set
            .find_syntax_by_token(syntax_hint)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut output = String::new();
        let mut highlighter = HighlightLines::new(syntax, theme);
        let c = Color {
            r: 249,
            g: 247,
            b: 236,
            a: 255,
        };
        write!(
            output,
            "<pre style=\"background-color:#{:02x}{:02x}{:02x};\">\n",
            c.r,
            c.g,
            c.b
        ).unwrap();
        for line in sample.lines() {
            let regions = highlighter.highlight(line);
            let html = styles_to_coloured_html(&regions[..], IncludeBackground::No);
            output.push_str(&html);
            output.push('\n');
        }
        output.push_str("</pre>\n");
        output
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

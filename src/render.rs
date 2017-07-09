use ammonia::Ammonia;
use pulldown_cmark::Parser;
use pulldown_cmark::html;

use util::CargoResult;

/// Renders a markdown text to sanitized HTML. The returned text should not contain any harmful
/// HTML tag or attribute (such as iframe, onclick, onmouseover, etc.).
pub fn markdown_to_html(text: &str) -> CargoResult<String> {
    let mut rendered = String::with_capacity(text.len() * 3 / 2);
    let cleaner = Ammonia {
        keep_cleaned_elements: true,
        ..Ammonia::default()
    };
    let parser = Parser::new(text);
    html::push_html(&mut rendered, parser);
    Ok(cleaner.clean(&rendered))
}

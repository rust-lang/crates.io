use pulldown_cmark::Parser;
use pulldown_cmark::html;

use util::CargoResult;

pub fn markdown_to_html(raw: Option<&str>) -> CargoResult<String> {
    let text = raw.unwrap_or("");
    let mut rendered = String::with_capacity(text.len() * 3 / 2);
    let parser = Parser::new(&text);
    html::push_html(&mut rendered, parser);
    Ok(rendered)
}

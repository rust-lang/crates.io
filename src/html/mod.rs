use std::collections::HashMap;

use conduit::{Request, Response};

use util::{CargoResult, RequestUtils};

extern crate handlebars;
use self::handlebars::*;

use krate;

use db::RequestTransaction;

pub mod categories;

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("format_num", Box::new(format_num));
    handlebars.register_helper("embed_svg",  Box::new(embed_svg));
    handlebars.register_template_string("layout",     include_str!("layout.hbs"))?;
    handlebars.register_template_string("index",      include_str!("index.hbs"))?;
    handlebars.register_template_string("categories", include_str!("_categories.hbs"))?;
    handlebars.register_template_string("crates",     include_str!("_crates.hbs"))?;
    handlebars.register_template_string("keywords",   include_str!("_keywords.hbs"))?;

    let mut json = json!(&krate::metadata::summary_json(&*req.db_conn()?)?);
    json["current_user"] = json!({"id": 1, "name": "Sean Linsley"});
    // TODO: flash message
    // TODO: user image in header

    let html = handlebars.render("index", &json).expect("failed to render");

    Ok(req.html(&html))
}

fn format_num(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let param = h.param(0).expect("no argument passed").value();
    let rendered = format!("{}", param);
    try!(rc.writer.write(rendered.into_bytes().as_ref()));
    Ok(())
}

fn embed_svg(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let param = h.param(0).expect("no argument passed").value().to_string().replace("\"", "");

    let mut svgs = HashMap::new();
    svgs.insert("crate".to_string(),           include_str!("svgs/crate.svg").to_string());
    svgs.insert("button-download".to_string(), include_str!("svgs/button-download.svg").to_string());
    svgs.insert("download".to_string(),        include_str!("svgs/download.svg").to_string());
    svgs.insert("flag".to_string(),            include_str!("svgs/flag.svg").to_string());
    svgs.insert("lock".to_string(),            include_str!("svgs/lock.svg").to_string());
    svgs.insert("right-arrow".to_string(),     include_str!("svgs/right-arrow.svg").to_string());

    let svg = &svgs[&param];

    try!(rc.writer.write(svg.as_ref()));

    Ok(())
}

#![allow(unused_imports)]

use std::fs::File;
use std::io::Read;

use conduit::{Request, Response};

use util::{human, internal, CargoResult, ChainError, RequestUtils};

extern crate handlebars;
use self::handlebars::*;

use krate;

use serde_json;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use conduit::WriteBody;

use diesel::*;
use db::RequestTransaction;

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let html = include_str!("index.hbs");

    let mut handlebars = Handlebars::new();
    handlebars.register_helper("format_num", Box::new(format_num));
    handlebars.register_template_string("categories", include_str!("_categories.hbs"))?;
    handlebars.register_template_string("crates", include_str!("_crates.hbs"))?;
    handlebars.register_template_string("keywords", include_str!("_keywords.hbs"))?;

    let mut json = json!(&krate::metadata::summary_json(&*req.db_conn()?).unwrap());
    json["current_user"] = json!({"id": 1, "name": "Sean Linsley"});
    // TODO: flash message
    // TODO: SVG embed
    // TODO: global layout, that other pages can render into
    // TODO: user image in header

    let rendered = handlebars.template_render(html, &json).expect("failed to render");

    Ok(req.html(&String::from(rendered)))
}

fn format_num(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let param = h.param(0).expect("no argument passed").value();
    let rendered = format!("{}", param);
    try!(rc.writer.write(rendered.into_bytes().as_ref()));
    Ok(())
}

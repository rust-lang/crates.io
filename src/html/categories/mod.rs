use conduit::{Request, Response};

use std::collections::HashMap;
use util::{CargoResult, RequestUtils};

extern crate handlebars;
use self::handlebars::*;

use category;

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("embed_svg", Box::new(embed_svg));
    handlebars.register_helper("link_to",   Box::new(link_to));
    handlebars.register_helper("pluralize", Box::new(pluralize));
    handlebars.register_template_string("layout", include_str!("../layout.hbs"))?;
    handlebars.register_template_string("index",  include_str!("index.hbs"))?;

    let mut json = json!(&category::index_json(req)?);
    json["current_user"] = json!({"id": 1, "name": "Sean Linsley"});
    json["params"]       = json!(req.query());

    #[derive(Serialize)]
    struct Sort {key: String, name: String};

    let sorts = vec![
        Sort{key: "alpha".into(), name: "Alphabetical".into()},
        Sort{key: "crates".into(), name: "# Crates".into()},
    ];
    json["sorts"] = json!(sorts);

    let sort = req.query().get("sort").unwrap_or(&sorts.first().unwrap().key).to_string();
    json["current_sort"] = json!(sorts.iter().find(|s| s.key == sort).expect("invalid sort key").name);

    // duplicated with `category::index_json` :(
    let (offset, limit) = req.pagination(10, 100)?;
    json["current_page_start"] = json!(offset);
    json["current_page_end"] = json!(limit);

    // TODO: build pagination links
    // TODO: limit is 10 here and in `category::index_json`, but it's not actually being applied? (also a problem in production)

    let html = handlebars.render("index", &json).expect("failed to render");

    Ok(req.html(&html))
}

fn embed_svg(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let param = h.param(0).expect("no argument passed").value().to_string().replace("\"", "");

    let mut svgs = HashMap::new();
    svgs.insert("crate".to_string(),     include_str!("../svgs/crate.svg").to_string());
    svgs.insert("left-pag".to_string(),  include_str!("../svgs/left-pag.svg").to_string());
    svgs.insert("right-pag".to_string(), include_str!("../svgs/right-pag.svg").to_string());
    svgs.insert("sort".to_string(),      include_str!("../svgs/sort.svg").to_string());

    let svg = &svgs[&param];

    try!(rc.writer.write(svg.as_ref()));

    Ok(())
}

fn link_to(h: &Helper, r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    h.template().map(|t| t.render(r, rc)).unwrap_or(Ok(()))
}

fn pluralize(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let string = h.param(0).expect("no argument passed").value().to_string().replace("\"", "");
    let num: u32 = h.param(1).expect("number not passed").value().to_string().parse().expect("invalid number");

    let plural = if num == 1 { string } else { string + "s" };

    try!(rc.writer.write(plural.into_bytes().as_ref()));
    Ok(())
}

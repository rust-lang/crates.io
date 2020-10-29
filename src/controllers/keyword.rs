use super::prelude::*;

use crate::controllers::helpers::{pagination::Paginated, Paginate};
use crate::models::Keyword;
use crate::views::EncodableKeyword;

/// Handles the `GET /keywords` route.
pub fn index(req: &mut dyn RequestExt) -> EndpointResult {
    use crate::schema::keywords;

    let conn = req.db_conn()?;
    let query = req.query();
    let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");

    let mut query = keywords::table.into_boxed();

    if sort == "crates" {
        query = query.order(keywords::crates_cnt.desc());
    } else {
        query = query.order(keywords::keyword.asc());
    }

    let data: Paginated<Keyword> = query.paginate(&req.query())?.load(&*conn)?;
    let total = data.total();
    let kws = data.into_iter().map(Keyword::encodable).collect::<Vec<_>>();

    #[derive(Serialize)]
    struct R {
        keywords: Vec<EncodableKeyword>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        total: Option<i64>,
    }

    Ok(req.json(&R {
        keywords: kws,
        meta: Meta { total },
    }))
}

/// Handles the `GET /keywords/:keyword_id` route.
pub fn show(req: &mut dyn RequestExt) -> EndpointResult {
    let name = &req.params()["keyword_id"];
    let conn = req.db_conn()?;

    let kw = Keyword::find_by_keyword(&conn, name)?;

    #[derive(Serialize)]
    struct R {
        keyword: EncodableKeyword,
    }
    Ok(req.json(&R {
        keyword: kw.encodable(),
    }))
}

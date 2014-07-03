use civet::response;
use conduit::{Request, Response};
use conduit_router::RequestParams;
use html::Escape;
use html::form;
use pg::PostgresConnection;
use pg::error::PgDbError;
use pg::types::ToSql;
use std::collections::HashMap;
use std::io::{IoResult, MemReader};
use std::io;

use app::RequestApp;
use user::RequestUser;

pub fn setup(conn: &PostgresConnection) {
    conn.execute("DROP TABLE IF EXISTS packages", []).unwrap();
    conn.execute("CREATE TABLE packages (
                    id              SERIAL PRIMARY KEY,
                    name            VARCHAR NOT NULL,
                    slug            VARCHAR NOT NULL,
                    manifest        VARCHAR NOT NULL
                  )", []).unwrap();
    conn.execute("ALTER TABLE packages ADD CONSTRAINT \
                  unique_slug UNIQUE (slug)", []).unwrap();
}

pub fn index(req: &mut Request) -> IoResult<Response> {
    super::layout(|dst| {
        let conn = req.app().db();
        let stmt = conn.prepare("SELECT name, slug FROM packages").unwrap();
        try!(write!(dst, r#"
            <table>
                <tr>
                    <th>Package</th>
                    <th>Links</th>
                </tr>"#));
        for row in stmt.query([]).unwrap() {
            let name: String = row["name"];
            let slug: String = row["slug"];
            try!(write!(dst, r#"
                <tr>
                    <td>{}</td>
                    <td><a href="/packages/{}">[manifest]</a></td>
                </tr>
            "#,
            Escape(name.as_slice()),
            Escape(slug.as_slice())));
        }
        try!(write!(dst, r#"
            </table>
            <a href="/packages/new">[new package]</a>
        "#));

        match req.user() {
            Some(user) => try!(write!(dst, "hello! {}",
                                      Escape(user.email.as_slice()))),
            None => try!(write!(dst, "<br/>\
                <a href=\"/users/auth/github/authorize\">please log in</a>")),
        }

        Ok(())
    })
}

pub fn new(_req: &mut Request) -> IoResult<Response> {
    super::layout(|dst| {
        write!(dst, r#"
            <form action="/packages/new" method="post">
                <dl>
                    <dd><label>Name</label></dd>
                    <dt><input type="text" name="name"/></dt>
                    <dd><label>Manifest</label></dd>
                    <dt><textarea name="manifest"></textarea></dt>
                    <dd><input type="submit" name="submit"/></dd>
                </dl>
            </form>
        "#)
    })
}

pub fn create(req: &mut Request) -> IoResult<Response> {
    let form = match form::parse(try!(req.body().read_to_str()).as_slice()) {
        Some(map) => map,
        None => return Err(io::standard_error(io::OtherIoError)),
    };
    let manifest = match form.find_equiv(&"manifest") {
        Some(manifest) => manifest,
        None => return Err(io::standard_error(io::OtherIoError)),
    };
    let name = match form.find_equiv(&"name") {
        Some(name) => name,
        None => return Err(io::standard_error(io::OtherIoError)),
    };
    let slug = to_slug(name.as_slice());

    // TODO: don't unwrap this
    let conn = req.app().db();
    let err = conn.execute("INSERT INTO packages (name, slug, manifest) \
                            VALUES ($1, $2, $3)",
                           [&name.as_slice() as &ToSql,
                            &slug.as_slice() as &ToSql,
                            &manifest.as_slice() as &ToSql]);
    match err {
        Ok(..) => {}
        Err(PgDbError(ref e))
            if e.constraint.as_ref().map(|a| a.as_slice())
                == Some("unique_slug") =>
        {
            println!("duplicate slug");
            return new(req);
        }
        Err(e) => fail!("postgres error: {}", e),
    }

    let mut map = HashMap::new();
    map.insert("Location".to_string(), vec!["/".to_string()]);
    Ok(response(302i, map, MemReader::new(Vec::new())))
}

pub fn get(req: &mut Request) -> IoResult<Response> {
    let conn = req.app().db();
    let params = req.params();
    let stmt = conn.prepare("SELECT * FROM packages WHERE slug = $1 LIMIT 1")
                   .unwrap();

    let row = stmt.query([&params["id"].as_slice() as &ToSql]).unwrap()
                  .next().unwrap();
    super::layout(|dst| {
        let name: String = row["name"];
        let slug: String = row["slug"];
        try!(write!(dst, r#"
            <tr>
                <td>{}</td>
                <td><a href="/packages/{}">[manifest]</a></td>
            </tr>
        "#,
        Escape(name.as_slice()),
        Escape(slug.as_slice())));
        Ok(())
    })
}

fn to_slug(s: &str) -> String {
    let mut ret = String::new();
    for ch in s.chars() {
        match ch {
            'a'..'z' |
            '0'..'9' |
            '-' | '_' => ret.push_char(ch),
            'A'..'Z' => ret.push_char(ch.to_lowercase()),
            ' ' => ret.push_char('-'),

            _ => {}
        }
    }
    return ret;
}

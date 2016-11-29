// Sync available crate categories from `src/categories.txt`.
// Runs when the server is started.

use pg;
use env;
use util::errors::CargoResult;

pub fn sync() -> CargoResult<()> {
    let conn = pg::Connection::connect(&env("DATABASE_URL")[..],
                                             pg::TlsMode::None).unwrap();
    let tx = conn.transaction().unwrap();

    let categories = include_str!("./categories.txt");

    let slug_categories: Vec<_> = categories.lines().map(|c| {
        let mut parts = c.splitn(2, ' ');
        let slug = parts.next().expect("No slug found!");
        let name = parts.next().expect("No display name found!");
        (slug, name)
    }).collect();

    let insert = slug_categories.iter().map(|&(ref slug, ref name)| {
        format!("(LOWER('{}'), '{}')", slug, name)
    }).collect::<Vec<_>>().join(",");

    let in_clause = slug_categories.iter().map(|&(slug, _)| {
        format!("LOWER('{}')", slug)
    }).collect::<Vec<_>>().join(",");

    try!(tx.batch_execute(
        &format!(" \
            INSERT INTO categories (slug, category) \
            VALUES {} \
            ON CONFLICT (slug) DO UPDATE SET category = EXCLUDED.category; \
            DELETE FROM categories \
            WHERE slug NOT IN ({});",
            insert,
            in_clause
        )[..]
    ));
    tx.set_commit();
    tx.finish().unwrap();
    Ok(())
}

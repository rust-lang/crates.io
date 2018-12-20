// Sync available crate categories from `src/categories.toml`.
// Runs when the server is started.

use crate::{
    db,
    util::errors::{internal, CargoResult, ChainError},
};

use diesel::prelude::*;

#[derive(Debug)]
struct Category {
    slug: String,
    name: String,
    description: String,
}

impl Category {
    fn from_parent(
        slug: &str,
        name: &str,
        description: &str,
        parent: Option<&Category>,
    ) -> Category {
        match parent {
            Some(parent) => Category {
                slug: format!("{}::{}", parent.slug, slug),
                name: format!("{}::{}", parent.name, name),
                description: description.into(),
            },
            None => Category {
                slug: slug.into(),
                name: name.into(),
                description: description.into(),
            },
        }
    }
}

fn required_string_from_toml<'a>(toml: &'a toml::value::Table, key: &str) -> CargoResult<&'a str> {
    toml.get(key).and_then(toml::Value::as_str).chain_error(|| {
        internal(&format_args!(
            "Expected category TOML attribute '{}' to be a String",
            key
        ))
    })
}

fn optional_string_from_toml<'a>(toml: &'a toml::value::Table, key: &str) -> &'a str {
    toml.get(key).and_then(toml::Value::as_str).unwrap_or("")
}

fn categories_from_toml(
    categories: &toml::value::Table,
    parent: Option<&Category>,
) -> CargoResult<Vec<Category>> {
    let mut result = vec![];

    for (slug, details) in categories {
        let details = details
            .as_table()
            .chain_error(|| internal(&format_args!("category {} was not a TOML table", slug)))?;

        let category = Category::from_parent(
            slug,
            required_string_from_toml(details, "name")?,
            optional_string_from_toml(details, "description"),
            parent,
        );

        if let Some(categories) = details.get("categories") {
            let categories = categories.as_table().chain_error(|| {
                internal(&format_args!(
                    "child categories of {} were not a table",
                    slug
                ))
            })?;

            result.extend(categories_from_toml(categories, Some(&category))?);
        }

        result.push(category)
    }

    Ok(result)
}

pub fn sync(toml_str: &str) -> CargoResult<()> {
    let conn = db::connect_now().unwrap();
    sync_with_connection(toml_str, &conn)
}

pub fn sync_with_connection(toml_str: &str, conn: &PgConnection) -> CargoResult<()> {
    use crate::schema::categories::dsl::*;
    use diesel::dsl::all;
    use diesel::pg::upsert::excluded;

    let toml: toml::value::Table =
        toml::from_str(toml_str).expect("Could not parse categories toml");

    let to_insert = categories_from_toml(&toml, None)
        .expect("Could not convert categories from TOML")
        .into_iter()
        .map(|c| {
            (
                slug.eq(c.slug.to_lowercase()),
                category.eq(c.name),
                description.eq(c.description),
            )
        })
        .collect::<Vec<_>>();

    conn.transaction(|| {
        let slugs = diesel::insert_into(categories)
            .values(&to_insert)
            .on_conflict(slug)
            .do_update()
            .set((
                category.eq(excluded(category)),
                description.eq(excluded(description)),
            ))
            .returning(slug)
            .get_results::<String>(&*conn)?;

        diesel::delete(categories)
            .filter(slug.ne(all(slugs)))
            .execute(&*conn)?;
        Ok(())
    })
}

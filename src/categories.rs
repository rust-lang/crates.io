// Sync available crate categories from `src/categories.toml`.
// Runs when the server is started.

use diesel;
use diesel::prelude::*;
use toml;

use db;
use schema::categories;
use util::errors::{internal, CargoResult, ChainError};

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
        let details = details.as_table().chain_error(|| {
            internal(&format_args!("category {} was not a TOML table", slug))
        })?;

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

#[derive(Insertable, Debug)]
#[table_name = "categories"]
struct NewCategory {
    slug: String,
    category: String,
    description: String,
}

pub fn sync(toml_str: &str) -> CargoResult<()> {
    let conn = db::connect_now().unwrap();
    sync_with_connection(toml_str, &conn)
}

pub fn sync_with_connection(toml_str: &str, conn: &PgConnection) -> CargoResult<()> {
    use diesel::pg::upsert::*;
    use diesel::expression::dsl::all;

    let toml: toml::value::Table =
        toml::from_str(toml_str).expect("Could not parse categories toml");

    let categories = categories_from_toml(&toml, None)
        .expect("Could not convert categories from TOML")
        .into_iter()
        .map(|c| {
            NewCategory {
                slug: c.slug.to_lowercase(),
                category: c.name,
                description: c.description,
            }
        })
        .collect::<Vec<_>>();

    let to_insert = categories.on_conflict(
        categories::slug,
        do_update().set((
            categories::category.eq(excluded(categories::category)),
            categories::description.eq(excluded(categories::description)),
        )),
    );

    conn.transaction(|| {
        let slugs = diesel::insert(&to_insert)
            .into(categories::table)
            .returning(categories::slug)
            .get_results::<String>(&*conn)?;

        let to_delete = categories::table.filter(categories::slug.ne(all(slugs)));
        diesel::delete(to_delete).execute(&*conn)?;
        Ok(())
    })
}

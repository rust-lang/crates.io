// Sync available crate categories from `src/categories.toml`.
// Runs when the server is started.

use toml;

use db;
use util::errors::{CargoResult, ChainError, internal};

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
            Some(parent) => {
                Category {
                    slug: format!("{}::{}", parent.slug, slug),
                    name: format!("{}::{}", parent.name, name),
                    description: description.into(),
                }
            }
            None => {
                Category {
                    slug: slug.into(),
                    name: name.into(),
                    description: description.into(),
                }
            }
        }
    }
}

fn required_string_from_toml<'a>(toml: &'a toml::Table, key: &str) -> CargoResult<&'a str> {
    toml.get(key).and_then(toml::Value::as_str).chain_error(|| {
        internal(&format_args!(
            "Expected category TOML attribute '{}' to be a String",
            key
        ))
    })
}

fn optional_string_from_toml<'a>(toml: &'a toml::Table, key: &str) -> &'a str {
    toml.get(key).and_then(toml::Value::as_str).unwrap_or("")
}

fn categories_from_toml(
    categories: &toml::Table,
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

pub fn sync() -> CargoResult<()> {
    let conn = db::connect_now();
    let tx = conn.transaction().unwrap();

    let categories = include_str!("./categories.toml");
    let toml = toml::Parser::new(categories).parse().expect(
        "Could not parse categories.toml",
    );

    let categories =
        categories_from_toml(&toml, None).expect("Could not convert categories from TOML");

    for category in &categories {
        tx.execute(
            "\
             INSERT INTO categories (slug, category, description) \
             VALUES (LOWER($1), $2, $3) \
             ON CONFLICT (slug) DO UPDATE \
             SET category = EXCLUDED.category, \
             description = EXCLUDED.description;",
            &[&category.slug, &category.name, &category.description],
        )?;
    }

    let in_clause = categories
        .iter()
        .map(|category| format!("LOWER('{}')", category.slug))
        .collect::<Vec<_>>()
        .join(",");

    tx.execute(
        &format!(
            "\
             DELETE FROM categories \
             WHERE slug NOT IN ({});",
            in_clause
        ),
        &[],
    )?;
    tx.set_commit();
    tx.finish().unwrap();
    Ok(())
}

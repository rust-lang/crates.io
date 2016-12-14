// Sync available crate categories from `src/categories.toml`.
// Runs when the server is started.

use toml;
use pg;
use env;
use util::errors::{CargoResult, ChainError, internal};

struct Category {
    name: String,
    slug: String,
    description: String,
}

impl Category {
    fn concat(&self, child: &Category) -> Category {
        Category {
            name: format!("{}::{}", self.name, child.name),
            slug: format!("{}::{}", self.slug, child.slug),
            description: child.description.clone(),
        }
    }
}

fn concat_parent_and_child(parent: Option<&Category>, child: Category)
                           -> Category {
    parent.map(|p| p.concat(&child)).unwrap_or(child)
}

fn required_string_from_toml(toml: &toml::Table, key: &str)
                             -> CargoResult<String> {
    toml.get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .chain_error(|| {
            internal("Expected Category toml attribute to be a String")
        })
}

fn optional_string_from_toml(toml: &toml::Table, key: &str)
                             -> String {
    toml.get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn category_from_toml(toml: &toml::Value, parent: Option<&Category>)
                      -> CargoResult<Vec<Category>> {
    let toml = toml.as_table().chain_error(|| {
        internal("Category isn't a toml Table")
    })?;

    let category = Category {
        slug: required_string_from_toml(&toml, "slug")?,
        name: required_string_from_toml(&toml, "name")?,
        description: optional_string_from_toml(&toml, "description"),
    };

    let category = concat_parent_and_child(parent, category);

    let mut children: Vec<_> = toml.get("categories")
        .and_then(toml::Value::as_slice)
        .map(|children| {
            children.iter()
                .flat_map(|ref child| {
                    category_from_toml(child, Some(&category))
                        .expect("Could not create child from toml")
                }).collect()
        }).unwrap_or(Vec::new());

    children.push(category);

    Ok(children)
}

pub fn sync() -> CargoResult<()> {
    let conn = pg::Connection::connect(&env("DATABASE_URL")[..],
                                             pg::TlsMode::None).unwrap();
    let tx = conn.transaction().unwrap();

    let categories = include_str!("./categories.toml");
    let toml = toml::Parser::new(categories).parse().expect(
        "Could not parse categories.toml"
    );

    let categories = toml.get("categories")
                         .expect("No categories key found")
                         .as_slice()
                         .expect("Categories isn't a toml::Array");

    let categories: Vec<_> = categories
        .iter()
        .flat_map(|c| {
            category_from_toml(c, None)
                .expect("Categories from toml failed")
        }).collect();

    for category in categories.iter() {
        tx.execute("\
            INSERT INTO categories (slug, category, description) \
            VALUES (LOWER($1), $2, $3) \
            ON CONFLICT (slug) DO UPDATE \
                SET category = EXCLUDED.category, \
                    description = EXCLUDED.description;",
            &[&category.slug, &category.name, &category.description]
        )?;
    }

    let in_clause = categories.iter().map(|ref category| {
        format!("LOWER('{}')", category.slug)
    }).collect::<Vec<_>>().join(",");

    tx.execute(&format!("\
        DELETE FROM categories \
        WHERE slug NOT IN ({});",
        in_clause),
        &[]
    )?;
    tx.set_commit();
    tx.finish().unwrap();
    Ok(())
}

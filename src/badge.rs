use Model;
use krate::Crate;
use schema::badges;
use util::CargoResult;

use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use pg::GenericConnection;
use pg::rows::Row;
use serde_json;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(tag = "badge_type", content = "attributes")]
pub enum Badge {
    #[serde(rename = "travis-ci")]
    TravisCi {
        repository: String, branch: Option<String>,
    },
    #[serde(rename = "appveyor")]
    Appveyor {
        repository: String, branch: Option<String>, service: Option<String>,
    },
    #[serde(rename = "gitlab")]
    GitLab {
        repository: String, branch: Option<String>,
    },
}

#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug, Deserialize)]
pub struct EncodableBadge {
    pub badge_type: String,
    pub attributes: HashMap<String, String>,
}

impl Queryable<badges::SqlType, Pg> for Badge {
    type Row = (i32, String, serde_json::Value);

    fn build((_, badge_type, attributes): Self::Row) -> Self {
        let json = json!({"badge_type": badge_type, "attributes": attributes});
        serde_json::from_value(json)
            .expect("Invalid CI badge in the database")
    }
}

impl Model for Badge {
    fn from_row(row: &Row) -> Badge {
        let badge_type: String = row.get("badge_type");
        let attributes: serde_json::Value = row.get("attributes");
        let json = json!({"badge_type": badge_type, "attributes": attributes});
        serde_json::from_value(json)
            .expect("Invalid CI badge in the database")
    }
    fn table_name(_: Option<Badge>) -> &'static str { "badges" }
}

impl Badge {
    pub fn encodable(self) -> EncodableBadge {
        serde_json::from_value(serde_json::to_value(self).unwrap()).unwrap()
    }

    pub fn badge_type(&self) -> &'static str {
        match *self {
            Badge::TravisCi {..} => "travis-ci",
            Badge::Appveyor {..} => "appveyor",
            Badge::GitLab{..} => "gitlab",
        }
    }

    pub fn update_crate<'a>(conn: &PgConnection,
                            krate: &Crate,
                            badges: Option<&'a HashMap<String, HashMap<String, String>>>)
                            -> CargoResult<Vec<&'a str>> {
        use diesel::{insert, delete};

        #[derive(Insertable)]
        #[table_name="badges"]
        struct NewBadge<'a> {
            crate_id: i32,
            badge_type: &'a str,
            attributes: serde_json::Value,
        }

        let mut invalid_badges = vec![];
        let mut new_badges = vec![];

        if let Some(badges) = badges {
            for (k, v) in badges {
                let attributes_json = serde_json::to_value(v).unwrap();

                let json = json!({"badge_type": k, "attributes": attributes_json});
                if serde_json::from_value::<Badge>(json).is_ok() {
                    new_badges.push(NewBadge {
                        crate_id: krate.id,
                        badge_type: &**k,
                        attributes: attributes_json,
                    });
                } else {
                    invalid_badges.push(&**k);
                }
            }
        }

        conn.transaction(|| {
            delete(badges::table.filter(badges::crate_id.eq(krate.id))).execute(conn)?;
            insert(&new_badges).into(badges::table).execute(conn)?;
            Ok(invalid_badges)
        })
    }

    pub fn update_crate_old(conn: &GenericConnection,
                        krate: &Crate,
                        badges: HashMap<String, HashMap<String, String>>)
                        -> CargoResult<Vec<String>> {

        let mut invalid_badges = vec![];

        let badges: Vec<Badge> = badges.into_iter().filter_map(|(k, v)| {
            let json = json!({"badge_type": k, "attributes": v});
            serde_json::from_value(json)
                .map_err(|_| invalid_badges.push(k))
                .ok()
        }).collect();

        conn.execute("\
            DELETE FROM badges \
            WHERE crate_id = $1;",
            &[&krate.id]
        )?;

        for badge in badges {
            let json = serde_json::to_value(badge)?;
            conn.execute("\
                INSERT INTO badges (crate_id, badge_type, attributes) \
                VALUES ($1, $2, $3) \
                ON CONFLICT (crate_id, badge_type) DO UPDATE \
                    SET attributes = EXCLUDED.attributes;",
                &[&krate.id, &json["badge_type"].as_str(), &json["attributes"]]
            )?;
        }
        Ok(invalid_badges)
    }
}

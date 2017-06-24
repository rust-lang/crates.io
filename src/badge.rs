use krate::Crate;
use schema::badges;
use util::CargoResult;

use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use serde_json;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(tag = "badge_type", content = "attributes")]
pub enum Badge {
    #[serde(rename = "travis-ci")]
    TravisCi {
        repository: String,
        branch: Option<String>,
    },
    #[serde(rename = "appveyor")]
    Appveyor {
        repository: String,
        branch: Option<String>,
        service: Option<String>,
    },
    #[serde(rename = "gitlab")]
    GitLab {
        repository: String,
        branch: Option<String>,
    },
    #[serde(rename = "is-it-maintained-issue-resolution")]
    IsItMaintainedIssueResolution { repository: String },
    #[serde(rename = "is-it-maintained-open-issues")]
    IsItMaintainedOpenIssues { repository: String },
    #[serde(rename = "codecov")]
    Codecov {
        repository: String,
        branch: Option<String>,
        service: Option<String>,
    },
    #[serde(rename = "coveralls")]
    Coveralls {
        repository: String,
        branch: Option<String>,
        service: Option<String>,
    },
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EncodableBadge {
    pub badge_type: String,
    pub attributes: HashMap<String, Option<String>>,
}

impl Queryable<badges::SqlType, Pg> for Badge {
    type Row = (i32, String, serde_json::Value);

    fn build((_, badge_type, attributes): Self::Row) -> Self {
        let json = json!({"badge_type": badge_type, "attributes": attributes});
        serde_json::from_value(json).expect("Invalid CI badge in the database")
    }
}

impl Badge {
    pub fn encodable(self) -> EncodableBadge {
        serde_json::from_value(serde_json::to_value(self).unwrap()).unwrap()
    }

    pub fn badge_type(&self) -> &'static str {
        match *self {
            Badge::TravisCi { .. } => "travis-ci",
            Badge::Appveyor { .. } => "appveyor",
            Badge::GitLab { .. } => "gitlab",
            Badge::IsItMaintainedIssueResolution { .. } => "is-it-maintained-issue-resolution",
            Badge::IsItMaintainedOpenIssues { .. } => "is-it-maintained-open-issues",
            Badge::Codecov { .. } => "codecov",
            Badge::Coveralls { .. } => "coveralls",
        }
    }

    pub fn update_crate<'a>(
        conn: &PgConnection,
        krate: &Crate,
        badges: Option<&'a HashMap<String, HashMap<String, String>>>,
    ) -> CargoResult<Vec<&'a str>> {
        use diesel::{insert, delete};

        #[derive(Insertable)]
        #[table_name = "badges"]
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
            delete(badges::table.filter(badges::crate_id.eq(krate.id)))
                .execute(conn)?;
            insert(&new_badges).into(badges::table).execute(conn)?;
            Ok(invalid_badges)
        })
    }
}

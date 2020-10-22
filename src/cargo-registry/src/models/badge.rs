use diesel::associations::HasTable;
use diesel::pg::Pg;
use diesel::prelude::*;
use std::collections::HashMap;

use crate::models::Crate;
use crate::schema::badges;
use crate::views::EncodableBadge;

/// A combination of a `Badge` and a crate ID.
///
/// We don't typically care about the crate ID when dealing with badges.
/// However, when we are eager loading all badges for a group of crates, we need
/// the crate ID to group badges by their owner.
#[derive(Debug, Queryable, Associations)]
#[belongs_to(Crate)]
#[table_name = "badges"]
pub struct CrateBadge {
    pub crate_id: i32,
    pub badge: Badge,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "badge_type", content = "attributes")]
pub enum Badge {
    TravisCi {
        repository: String,
        branch: Option<String>,
    },
    Appveyor {
        repository: String,
        id: Option<String>,
        branch: Option<String>,
        project_name: Option<String>,
        service: Option<String>,
    },
    AzureDevops {
        project: String,
        pipeline: String,
        build: Option<String>,
    },
    #[serde(rename = "gitlab")]
    GitLab {
        repository: String,
        branch: Option<String>,
    },
    CircleCi {
        repository: String,
        branch: Option<String>,
    },
    CirrusCi {
        repository: String,
        branch: Option<String>,
    },
    IsItMaintainedIssueResolution {
        repository: String,
    },
    IsItMaintainedOpenIssues {
        repository: String,
    },
    Codecov {
        repository: String,
        branch: Option<String>,
        service: Option<String>,
    },
    Coveralls {
        repository: String,
        branch: Option<String>,
        service: Option<String>,
    },
    BitbucketPipelines {
        repository: String,
        branch: String,
    },
    Maintenance {
        status: MaintenanceStatus,
    },
}

#[derive(Debug, PartialEq, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaintenanceStatus {
    ActivelyDeveloped,
    PassivelyMaintained,
    AsIs,
    None,
    Experimental,
    LookingForMaintainer,
    Deprecated,
}

// FIXME: Derive this in Diesel 1.4.
impl HasTable for CrateBadge {
    type Table = badges::table;

    fn table() -> Self::Table {
        badges::table
    }
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
        // The serde attributes on Badge ensure it can be deserialized to EncodableBadge
        serde_json::from_value(serde_json::to_value(self).unwrap()).unwrap()
    }

    pub fn update_crate(
        conn: &PgConnection,
        krate: &Crate,
        badges: Option<&HashMap<String, HashMap<String, String>>>,
    ) -> QueryResult<Vec<String>> {
        use diesel::{delete, insert_into};

        let mut invalid_badges = vec![];
        let mut new_badges = vec![];

        if let Some(badges) = badges {
            for (k, v) in badges {
                let attributes_json = serde_json::to_value(v).unwrap();

                let json = json!({"badge_type": k, "attributes": attributes_json});
                if serde_json::from_value::<Badge>(json).is_ok() {
                    new_badges.push((
                        badges::crate_id.eq(krate.id),
                        badges::badge_type.eq(k),
                        badges::attributes.eq(attributes_json),
                    ));
                } else {
                    invalid_badges.push(k.to_string());
                }
            }
        }

        conn.transaction(|| {
            delete(badges::table)
                .filter(badges::crate_id.eq(krate.id))
                .execute(conn)?;
            insert_into(badges::table)
                .values(&new_badges)
                .execute(conn)?;
            Ok(invalid_badges)
        })
    }
}

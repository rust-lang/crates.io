use Model;
use krate::Crate;
use schema::badges;
use util::CargoResult;

use diesel::pg::Pg;
use diesel::prelude::*;
use pg::GenericConnection;
use pg::rows::Row;
use rustc_serialize::Decodable;
use rustc_serialize::json::{Json, Decoder};
use serde_json;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum Badge {
    TravisCi {
        repository: String, branch: Option<String>,
    },
    Appveyor {
        repository: String, branch: Option<String>, service: Option<String>,
    },
    GitLab {
        repository: String, branch: Option<String>,
    },
}

#[derive(RustcEncodable, RustcDecodable, PartialEq, Debug)]
pub struct EncodableBadge {
    pub badge_type: String,
    pub attributes: HashMap<String, String>,
}

impl Queryable<badges::SqlType, Pg> for Badge {
    type Row = (i32, String, serde_json::Value);

    fn build((_, badge_type, attributes): Self::Row) -> Self {
        let attributes = serde_json::from_value::<HashMap<String, String>>(attributes)
            .expect("attributes was not a map in the database");
        Self::from_attributes(&badge_type, &attributes)
            .expect("invalid badge in the database")
    }
}

impl Model for Badge {
    fn from_row(row: &Row) -> Badge {
        let attributes: Json = row.get("attributes");
        let badge_type: String = row.get("badge_type");
        let mut decoder = Decoder::new(attributes);
        let attributes = HashMap::<String, String>::decode(&mut decoder)
            .expect("Attributes was not a json object");
        Self::from_attributes(&badge_type, &attributes)
            .expect("Invalid CI badge in the database")
    }
    fn table_name(_: Option<Badge>) -> &'static str { "badges" }
}

impl Badge {
    pub fn encodable(self) -> EncodableBadge {
        EncodableBadge {
            badge_type: self.badge_type().to_string(),
            attributes: self.attributes(),
        }
    }

    pub fn badge_type(&self) -> &'static str {
        match *self {
            Badge::TravisCi {..} => "travis-ci",
            Badge::Appveyor {..} => "appveyor",
            Badge::GitLab{..} => "gitlab",
        }
    }

    pub fn json_attributes(self) -> Json {
        Json::Object(self.attributes().into_iter().map(|(k, v)| {
            (k, Json::String(v))
        }).collect())
    }

    fn attributes(self) -> HashMap<String, String> {
        let mut attributes = HashMap::new();

        match self {
            Badge::TravisCi { branch, repository } => {
                attributes.insert(String::from("repository"), repository);
                if let Some(branch) = branch {
                    attributes.insert(
                        String::from("branch"),
                        branch
                    );
                }
            },
            Badge::Appveyor { service, branch, repository } => {
                attributes.insert(String::from("repository"), repository);
                if let Some(branch) = branch {
                    attributes.insert(
                        String::from("branch"),
                        branch
                    );
                }
                if let Some(service) = service {
                    attributes.insert(
                        String::from("service"),
                        service
                    );
                }
            },
            Badge::GitLab { branch, repository } => {
                attributes.insert(String::from("repository"), repository);
                if let Some(branch) = branch {
                    attributes.insert(
                        String::from("branch"),
                        branch
                    );
                }
            },
        }

        attributes
    }

    fn from_attributes(badge_type: &str,
                       attributes: &HashMap<String, String>)
                       -> Result<Badge, String> {
        match badge_type {
            "travis-ci" => {
                match attributes.get("repository") {
                    Some(repository) => {
                        Ok(Badge::TravisCi {
                            repository: repository.to_string(),
                            branch: attributes.get("branch")
                                              .map(String::to_string),
                        })
                    },
                    None => Err(badge_type.to_string()),
                }
            },
            "appveyor" => {
                match attributes.get("repository") {
                    Some(repository) => {
                        Ok(Badge::Appveyor {
                            repository: repository.to_string(),
                            branch: attributes.get("branch")
                                              .map(String::to_string),
                            service: attributes.get("service")
                                              .map(String::to_string),

                        })
                    },
                    None => Err(badge_type.to_string()),
                }
            },
            "gitlab" => {
                match attributes.get("repository") {
                    Some(repository) => {
                        Ok(Badge::GitLab {
                            repository: repository.to_string(),
                            branch: attributes.get("branch")
                                              .map(String::to_string),
                        })
                    },
                    None => Err(badge_type.to_string()),
                }
            },
           _ => Err(badge_type.to_string()),
        }
    }

    pub fn update_crate(conn: &GenericConnection,
                        krate: &Crate,
                        badges: HashMap<String, HashMap<String, String>>)
                        -> CargoResult<Vec<String>> {

        let mut invalid_badges = vec![];

        let badges: Vec<_> = badges.iter().filter_map(|(k, v)| {
            Badge::from_attributes(k, v).map_err(|invalid_badge| {
                invalid_badges.push(invalid_badge)
            }).ok()
        }).collect();

        conn.execute("\
            DELETE FROM badges \
            WHERE crate_id = $1;",
            &[&krate.id]
        )?;

        for badge in badges {
            conn.execute("\
                INSERT INTO badges (crate_id, badge_type, attributes) \
                VALUES ($1, $2, $3) \
                ON CONFLICT (crate_id, badge_type) DO UPDATE \
                    SET attributes = EXCLUDED.attributes;",
                &[&krate.id, &badge.badge_type(), &badge.json_attributes()]
            )?;
        }
        Ok(invalid_badges)
    }
}

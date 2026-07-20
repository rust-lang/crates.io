use docs_rs_crates_io::events::{CrateVersion, IndexChangeV1};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Trigger {
    Added { version: String },
    Unyanked { version: String },
    Yanked { version: String },
    CrateDeleted,
    VersionsDeleted { versions: Vec<String> },
}

impl Trigger {
    pub fn into_iter<'a>(self, name: &'a str) -> TriggerIterator<'a> {
        TriggerIterator {
            name,
            trigger: Some(self),
        }
    }
}

pub struct TriggerIterator<'a> {
    name: &'a str,
    trigger: Option<Trigger>,
}

impl<'a> Iterator for TriggerIterator<'a> {
    type Item = IndexChangeV1;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(trigger) = self.trigger.take() {
            let name = self.name.to_owned();

            match trigger {
                Trigger::Added { version } => {
                    Some(IndexChangeV1::Added(CrateVersion { name, version }))
                }
                Trigger::Unyanked { version } => {
                    Some(IndexChangeV1::Unyanked(CrateVersion { name, version }))
                }
                Trigger::Yanked { version } => {
                    Some(IndexChangeV1::Yanked(CrateVersion { name, version }))
                }
                Trigger::CrateDeleted => Some(IndexChangeV1::CrateDeleted { name }),
                Trigger::VersionsDeleted { mut versions } => {
                    if let Some(version) = versions.pop() {
                        let item = IndexChangeV1::VersionDeleted(CrateVersion { name, version });
                        self.trigger.replace(Trigger::VersionsDeleted { versions });

                        Some(item)
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        }
    }
}

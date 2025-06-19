use crate::auth::Permission;
use crate::util::errors::{BoxedAppError, forbidden};

pub struct AuthorizedTrustPub {
    crate_ids: Vec<i32>,
}

impl AuthorizedTrustPub {
    pub fn new(crate_ids: Vec<i32>) -> Self {
        Self { crate_ids }
    }

    pub(in crate::auth) async fn validate(
        self,
        permission: Permission<'_>,
    ) -> Result<Self, BoxedAppError> {
        let existing_crate = match permission {
            Permission::PublishUpdate { krate } => krate,
            Permission::PublishNew { .. } => {
                let message = "Trusted Publishing tokens do not support creating new crates";
                return Err(forbidden(message));
            }
            _ => {
                let message = "Trusted Publishing tokens can only be used for publishing crates";
                return Err(forbidden(message));
            }
        };

        if !self.crate_ids.contains(&existing_crate.id) {
            let name = &existing_crate.name;
            let error = format!("The provided access token is not valid for crate `{name}`");
            return Err(forbidden(error));
        }

        Ok(self)
    }
}

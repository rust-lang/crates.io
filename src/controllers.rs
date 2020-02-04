mod cargo_prelude {
    pub use super::prelude::*;
    pub use crate::util::errors::cargo_err;
}

mod frontend_prelude {
    pub use super::prelude::*;
    pub use crate::util::errors::{bad_request, server_error};
}

mod prelude {
    pub use super::helpers::ok_true;
    pub use diesel::prelude::*;

    pub use conduit::{Request, Response};
    pub use conduit_router::RequestParams;

    pub use crate::db::RequestTransaction;
    pub use crate::util::errors::{cargo_err, AppError, AppResult, ChainError}; // TODO: Remove cargo_err from here

    pub use crate::middleware::app::RequestApp;

    use std::collections::HashMap;
    use std::io;

    use indexmap::IndexMap;
    use serde::Serialize;
    use url;

    pub trait UserAuthenticationExt {
        fn authenticate(&self, conn: &PgConnection) -> AppResult<super::util::AuthenticatedUser>;
    }

    pub trait RequestUtils {
        fn redirect(&self, url: String) -> Response;

        fn json<T: Serialize>(&self, t: &T) -> Response;
        fn query(&self) -> IndexMap<String, String>;
        fn wants_json(&self) -> bool;
        fn query_with_params(&self, params: IndexMap<String, String>) -> String;
    }

    impl<'a> RequestUtils for dyn Request + 'a {
        fn json<T: Serialize>(&self, t: &T) -> Response {
            crate::util::json_response(t)
        }

        fn query(&self) -> IndexMap<String, String> {
            url::form_urlencoded::parse(self.query_string().unwrap_or("").as_bytes())
                .into_owned()
                .collect()
        }

        fn redirect(&self, url: String) -> Response {
            let mut headers = HashMap::new();
            headers.insert("Location".to_string(), vec![url]);
            Response {
                status: (302, "Found"),
                headers,
                body: Box::new(io::empty()),
            }
        }

        fn wants_json(&self) -> bool {
            self.headers()
                .find("Accept")
                .map(|accept| accept.iter().any(|s| s.contains("json")))
                .unwrap_or(false)
        }

        fn query_with_params(&self, new_params: IndexMap<String, String>) -> String {
            let mut params = self.query();
            params.extend(new_params);
            let query_string = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(params)
                .finish();
            format!("?{}", query_string)
        }
    }
}

pub mod helpers;
mod util;

pub mod category;
pub mod crate_owner_invitation;
pub mod keyword;
pub mod krate;
pub mod site_metadata;
pub mod team;
pub mod token;
pub mod user;
pub mod version;

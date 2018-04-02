mod prelude {
    pub use diesel::prelude::*;
    pub use super::helpers::ok_true;

    pub use conduit::{Request, Response};
    pub use conduit_router::RequestParams;

    pub use db::RequestTransaction;
    pub use util::{human, CargoResult};

    pub use middleware::app::RequestApp;
    pub use middleware::current_user::RequestUser;

    use std::io;
    use url;
    use std::collections::HashMap;
    use serde::Serialize;

    pub trait RequestUtils {
        fn redirect(&self, url: String) -> Response;

        fn json<T: Serialize>(&self, t: &T) -> Response;
        fn query(&self) -> HashMap<String, String>;
        fn wants_json(&self) -> bool;
        fn pagination(&self, default: usize, max: usize) -> CargoResult<(i64, i64)>;
    }

    impl<'a> RequestUtils for Request + 'a {
        fn json<T: Serialize>(&self, t: &T) -> Response {
            ::util::json_response(t)
        }

        fn query(&self) -> HashMap<String, String> {
            url::form_urlencoded::parse(self.query_string().unwrap_or("").as_bytes())
                .map(|(a, b)| (a.into_owned(), b.into_owned()))
                .collect()
        }

        fn redirect(&self, url: String) -> Response {
            let mut headers = HashMap::new();
            headers.insert("Location".to_string(), vec![url.to_string()]);
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

        fn pagination(&self, default: usize, max: usize) -> CargoResult<(i64, i64)> {
            let query = self.query();
            let page = query
                .get("page")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let limit = query
                .get("per_page")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(default);
            if limit > max {
                return Err(human(&format_args!(
                    "cannot request more than {} items",
                    max
                )));
            }
            if page == 0 {
                return Err(human("page indexing starts from 1, page 0 is invalid"));
            }
            Ok((((page - 1) * limit) as i64, limit as i64))
        }
    }
}

pub mod helpers;

pub mod category;
pub mod crate_owner_invitation;
pub mod keyword;
pub mod krate;
pub mod site_metadata;
pub mod team;
pub mod token;
pub mod version;
pub mod user;

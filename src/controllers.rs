mod cargo_prelude {
    pub use super::prelude::*;
}

mod frontend_prelude {
    pub use super::prelude::*;
    pub use crate::util::errors::{bad_request, server_error};
}

mod prelude {
    pub use super::helpers::ok_true;
    pub use axum::extract::Path;
    pub use axum::response::{IntoResponse, Response};
    pub use axum::Json;
    pub use diesel::prelude::*;
    pub use serde_json::Value;

    pub use http::{header, request::Parts, StatusCode};

    pub use crate::app::AppState;
    pub use crate::middleware::app::RequestApp;
    pub use crate::tasks::spawn_blocking;
    pub use crate::util::errors::{AppResult, BoxedAppError};
    pub use crate::util::{redirect, BytesRequest, RequestUtils};
}

pub mod helpers;
pub mod util;

pub mod category;
pub mod crate_owner_invitation;
pub mod git;
pub mod github;
pub mod keyword;
pub mod krate;
pub mod metrics;
pub mod site_metadata;
pub mod summary;
pub mod team;
pub mod token;
pub mod user;
pub mod version;

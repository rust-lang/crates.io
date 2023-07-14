use axum::response::IntoResponse;
use minijinja::context;

use crate::{
    controllers::helpers::pagination::Paginated,
    models::{Crate, User, Version},
};

use super::templating::{self, components, Renderer};

#[derive(Serialize)]
struct View {
    page: components::Page,
    q: String,
    versions: Vec<CrateVersion>,
}

#[derive(Serialize)]
struct CrateVersion {
    id: i32,
    name: String,
    num: String,
    created_at: components::DateTime,
    publisher: components::User,
    yanked: bool,
}

impl CrateVersion {
    fn new(version: Version, krate: Crate, user: User) -> Self {
        Self {
            id: version.id,
            name: krate.name,
            num: version.num,
            created_at: version.created_at.into(),
            publisher: user.into(),
            yanked: version.yanked,
        }
    }
}

pub fn render<R>(
    env: &R,
    q: Option<&str>,
    page: Paginated<(Version, Crate, User)>,
) -> impl IntoResponse
where
    R: Renderer,
{
    let view = View {
        q: q.map(|s| s.to_string()).unwrap_or_default(),
        page: components::Page::new(&page, q),
        versions: page
            .into_iter()
            .map(|(version, krate, user)| CrateVersion::new(version, krate, user))
            .collect(),
    };

    templating::render_response(env, "crates/index.html", context!(view => view))
}

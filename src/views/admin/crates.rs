use axum::response::IntoResponse;
use axum_template::{engine::Engine, RenderHtml};
use handlebars::Handlebars;

use crate::{
    controllers::helpers::pagination::Paginated,
    models::{Crate, User, Version},
};

use super::templating::components;

#[derive(Serialize)]
struct View {
    page: components::Page,
    q: Option<String>,
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

pub fn render(
    engine: &Engine<Handlebars<'static>>,
    q: Option<&str>,
    page: Paginated<(Version, Crate, User)>,
) -> impl IntoResponse {
    RenderHtml(
        "crates",
        engine.clone(),
        View {
            q: q.map(|s| s.to_string()),
            page: components::Page::new(&page, q),
            versions: page
                .into_iter()
                .map(|(version, krate, user)| CrateVersion::new(version, krate, user))
                .collect(),
        },
    )
}

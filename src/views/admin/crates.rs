use axum::response::IntoResponse;
use axum_template::{engine::Engine, RenderHtml};
use handlebars::Handlebars;

use crate::models::{Crate, User, Version};

use super::templating::components;

#[derive(Serialize)]
pub struct CrateVersion {
    pub id: i32,
    pub name: String,
    pub num: String,
    pub created_at: components::DateTime,
    pub publisher: components::User,
    pub yanked: bool,
}

impl CrateVersion {
    pub fn new(version: Version, krate: Crate, user: User) -> Self {
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

pub fn render_versions(
    engine: &Engine<Handlebars<'static>>,
    iter: impl Iterator<Item = CrateVersion>,
) -> impl IntoResponse {
    RenderHtml(
        "crates",
        engine.clone(),
        iter.collect::<Vec<CrateVersion>>(),
    )
}

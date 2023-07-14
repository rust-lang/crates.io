use axum::response::{IntoResponse, Response};
use http::{header, StatusCode};
use minijinja::Environment;
use serde::Serialize;
use thiserror::Error;

pub mod components;
mod helpers;

// If this all feels overly complicated, it is.
//
// The goal here is simple. In debug builds, we want to hot reload templates as they are changed on
// the disk, since that makes template development easier. In release builds, we want to embed the
// templates into the binary, compile them once on startup, and then never think about them again
// except to use them for rendering.
//
// The only interface we need at the view level is the ability to render an arbitrary template with
// arbitrary context data.
//
// Unfortunately, the underlying types to facilitate this don't look much like each other. The
// debug version is AutoReloader (no lifetime), while the release version is Environment<'source>
// (complete with lifetime). We can't just box this behind a trait because the render methods
// aren't object safe, since the context data can be anything serde can serialise.
//
// Instead, we'll do some compile time detection to make the debug and release implementations look
// similar enough that they can be used interchangably by calling code, and provide a trait that
// they both implement that smoothes over the differences.

pub trait Renderer {
    fn render<S>(&self, key: &str, data: S) -> Result<String, Error>
    where
        S: Serialize;
}

#[cfg(debug_assertions)]
pub type Engine = debug::Engine;

#[cfg(not(debug_assertions))]
pub type Engine = release::Engine<'static>;

pub fn engine() -> Result<Engine, Error> {
    Engine::new()
}

pub fn render_response<R, S>(env: &R, key: &str, data: S) -> Response
where
    R: Renderer,
    S: Serialize,
{
    match env.render(key, data) {
        Ok(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content,
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

mod debug {
    use std::{path::PathBuf, str::FromStr};

    use minijinja::{path_loader, Environment};
    use minijinja_autoreload::AutoReloader;

    use super::{register_filters, Error, Renderer};

    pub struct Engine(AutoReloader);

    impl Engine {
        #[allow(dead_code)]
        pub(super) fn new() -> Result<Self, Error> {
            let template_path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
                .expect("PathBuf::from_str is infallible")
                .join("admin")
                .join("templates");

            Ok(Self(AutoReloader::new(move |notifier| {
                notifier.watch_path(&template_path, true);

                let mut env = Environment::new();
                env.set_debug(true);
                env.set_loader(path_loader(&template_path));
                register_filters(&mut env);
                Ok(env)
            })))
        }
    }

    impl Renderer for Engine {
        fn render<S>(&self, key: &str, data: S) -> Result<String, Error>
        where
            S: serde::Serialize,
        {
            Ok(self.0.acquire_env()?.get_template(key)?.render(data)?)
        }
    }
}

mod release {
    use minijinja::Environment;
    use rust_embed::RustEmbed;
    use serde::Serialize;

    use super::{register_filters, Error, Renderer};

    #[derive(RustEmbed)]
    #[folder = "admin/templates/"]
    struct Assets;

    pub struct Engine<'a>(Environment<'a>);

    impl<'a> Engine<'a> {
        #[allow(dead_code)]
        pub(super) fn new() -> Result<Self, Error> {
            let mut env = Environment::new();

            for name in Assets::iter() {
                env.add_template_owned(
                    name.to_string(),
                    String::from_utf8(Assets::get(&name).expect("embedded template").data.to_vec())
                        .expect("template must be valid UTF-8"),
                )?;
            }

            register_filters(&mut env);
            Ok(Self(env))
        }
    }

    impl<'a> Renderer for Engine<'a> {
        fn render<S>(&self, key: &str, data: S) -> Result<String, Error>
        where
            S: Serialize,
        {
            Ok(self.0.get_template(key)?.render(data)?)
        }
    }
}

fn register_filters(env: &mut Environment<'_>) {
    env.add_filter("crate_index_path", helpers::crate_index_path);
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Minijinja(#[from] minijinja::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        // FIXME: don't necessarily output the full error in prod.
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", &self)).into_response()
    }
}

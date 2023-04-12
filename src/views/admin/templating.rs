use std::string::FromUtf8Error;

use axum_template::engine::Engine;
use handlebars::{handlebars_helper, Handlebars, TemplateError};
use thiserror::Error;

#[derive(rust_embed::RustEmbed)]
#[folder = "admin/templates/"]
struct Assets;

pub fn engine() -> Result<Engine<Handlebars<'static>>, Error> {
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(true);

    #[cfg(debug_assertions)]
    configure_handlebars_debug(&mut hbs)?;

    #[cfg(not(debug_assertions))]
    configure_handlebars_release(&mut hbs)?;

    handlebars_helper!(crate_index_path: |name: str| helpers::crate_index_path(name));
    hbs.register_helper("crate-index-path", Box::new(crate_index_path));

    Ok(Engine::from(hbs))
}

#[allow(dead_code)]
fn configure_handlebars_debug(hbs: &mut Handlebars<'_>) -> Result<(), Error> {
    hbs.set_dev_mode(true);
    hbs.register_templates_directory(".hbs", "admin/templates")?;
    Ok(())
}

#[allow(dead_code)]
fn configure_handlebars_release(hbs: &mut Handlebars<'_>) -> Result<(), Error> {
    // RustEmbed doesn't strip the `.hbs` extension from each template when
    // using `Handlebars::register_embed_templates`, whereas
    // `Handlebars::register_templates_directory` does. We'll walk the assets
    // and register them individually to strip the extensions.
    //
    // In theory, we could do it the other way around in debug mode, but then
    // the dev mode auto-reload wouldn't discover new templates without
    // restarting the service.
    for file in Assets::iter() {
        let content = String::from_utf8(
            Assets::get(&file)
                .ok_or_else(|| Error::MissingFile(file.to_string()))?
                .data
                .into_owned(),
        )
        .map_err(|e| Error::MalformedContent(file.to_string(), e))?;

        let name = file
            .strip_suffix(".hbs")
            .ok_or_else(|| Error::UnexpectedExtension(file.to_string()))?;

        hbs.register_template_string(name, content)?;
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("template {0} content is not valid UTF-8")]
    MalformedContent(String, #[source] FromUtf8Error),

    #[error("missing template file {0}")]
    MissingFile(String),

    #[error(transparent)]
    Template(#[from] Box<TemplateError>),

    #[error("template file {0} does not have the expected .hbs extension")]
    UnexpectedExtension(String),
}

impl From<TemplateError> for Error {
    fn from(value: TemplateError) -> Self {
        // TemplateError is big, so we box it to avoid clippy warnings.
        Self::Template(Box::new(value))
    }
}

mod helpers {
    use cargo_registry_index::Repository;

    pub(super) fn crate_index_path(name: &str) -> String {
        String::from(
            Repository::relative_index_file(name)
                .to_str()
                .expect("invalid UTF-8 in crate name"),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use anyhow::Error;

    #[test]
    fn template_names_match_in_debug_and_release_modes() -> Result<(), Error> {
        let mut debug = Handlebars::new();
        debug.set_strict_mode(true);
        configure_handlebars_debug(&mut debug)?;

        let mut release = Handlebars::new();
        release.set_strict_mode(true);
        configure_handlebars_release(&mut release)?;

        let debug_templates: HashSet<&String> = debug.get_templates().keys().collect();
        let release_templates: HashSet<&String> = release.get_templates().keys().collect();

        assert_eq!(debug_templates, release_templates);

        Ok(())
    }
}

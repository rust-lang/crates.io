use crates_io_cargo_toml::{DepsSet, Error, Manifest, MaybeInherited, Package};

pub fn validate_manifest(manifest: &Manifest) -> Result<(), Error> {
    let package = manifest.package.as_ref();

    // Check that a `[package]` table exists in the manifest, since crates.io
    // does not accept workspace manifests.
    let package = package.ok_or(Error::Other("missing field `package`".to_string()))?;

    // We don't want to allow [patch] sections in manifests at all.
    if matches!(&manifest.patch, Some(patch) if !patch.is_empty()) {
        return Err(Error::Other(
            "crates cannot be published with `[patch]` tables".to_string(),
        ));
    }

    validate_package(package)?;

    // These checks ensure that dependency workspace inheritance has been
    // normalized by cargo before publishing.
    let has_inherited_dep =
        |deps: &Option<DepsSet>| deps.iter().flatten().any(|(_, dep)| dep.is_inherited());
    if has_inherited_dep(&manifest.dependencies)
        || has_inherited_dep(&manifest.dev_dependencies)
        || has_inherited_dep(&manifest.build_dependencies)
    {
        return Err(Error::Other(
            "value from workspace hasn't been set".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_package(package: &Package) -> Result<(), Error> {
    // Check that the `version` field exists in the package table.
    let version = package
        .version
        .as_ref()
        .ok_or(Error::Other("missing field `version`".to_string()))?;

    // These checks ensure that package field workspace inheritance has been
    // normalized by cargo before publishing.
    if version.is_inherited()
        || is_inherited(&package.edition)
        || is_inherited(&package.rust_version)
        || is_inherited(&package.authors)
        || is_inherited(&package.description)
        || is_inherited(&package.homepage)
        || is_inherited(&package.documentation)
        || is_inherited(&package.readme)
        || is_inherited(&package.keywords)
        || is_inherited(&package.categories)
        || is_inherited(&package.license)
        || is_inherited(&package.license_file)
        || is_inherited(&package.repository)
    {
        return Err(Error::Other(
            "value from workspace hasn't been set".to_string(),
        ));
    }

    Ok(())
}

fn is_inherited<T>(field: &Option<MaybeInherited<T>>) -> bool {
    field.as_ref().is_some_and(|it| it.is_inherited())
}

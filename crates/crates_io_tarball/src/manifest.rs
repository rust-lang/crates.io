use cargo_manifest::{Dependency, DepsSet, Error, Manifest, MaybeInherited, Package};

pub fn validate_manifest(manifest: &Manifest) -> Result<(), Error> {
    let package = manifest.package.as_ref();

    // Check that a `[package]` table exists in the manifest, since crates.io
    // does not accept workspace manifests.
    let package = package.ok_or(Error::Other("missing field `package`".to_string()))?;

    validate_package(package)?;

    // These checks ensure that dependency workspace inheritance has been
    // normalized by cargo before publishing.
    if manifest.dependencies.is_inherited()
        || manifest.dev_dependencies.is_inherited()
        || manifest.build_dependencies.is_inherited()
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
    if package.edition.is_inherited()
        || package.rust_version.is_inherited()
        || version.is_inherited()
        || package.authors.is_inherited()
        || package.description.is_inherited()
        || package.homepage.is_inherited()
        || package.documentation.is_inherited()
        || package.readme.is_inherited()
        || package.keywords.is_inherited()
        || package.categories.is_inherited()
        || package.exclude.is_inherited()
        || package.include.is_inherited()
        || package.license.is_inherited()
        || package.license_file.is_inherited()
        || package.repository.is_inherited()
        || package.publish.is_inherited()
    {
        return Err(Error::Other(
            "value from workspace hasn't been set".to_string(),
        ));
    }

    Ok(())
}

trait IsInherited {
    fn is_inherited(&self) -> bool;
}

impl<T> IsInherited for MaybeInherited<T> {
    fn is_inherited(&self) -> bool {
        matches!(self, MaybeInherited::Inherited { .. })
    }
}

impl<T: IsInherited> IsInherited for Option<T> {
    fn is_inherited(&self) -> bool {
        self.as_ref().map(|it| it.is_inherited()).unwrap_or(false)
    }
}

impl IsInherited for Dependency {
    fn is_inherited(&self) -> bool {
        matches!(self, Dependency::Inherited(_))
    }
}

impl IsInherited for DepsSet {
    fn is_inherited(&self) -> bool {
        self.iter().any(|(_key, dep)| dep.is_inherited())
    }
}

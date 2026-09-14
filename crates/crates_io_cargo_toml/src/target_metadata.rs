use crate::{Error as ManifestError, Manifest, PathsFileSystem, Product, StringOrBool};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Metadata describing a package source file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceFile {
    /// The path relative to the package root.
    pub path: String,
    /// Whether the file is present in the package source inventory.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub exists: bool,
}

/// Metadata describing a library target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LibraryMetadata {
    /// The library source file.
    #[serde(flatten)]
    pub source: SourceFile,
    /// The target name used by Cargo.
    pub name: String,
    /// Whether the library is a procedural macro.
    pub is_proc_macro: bool,
}

/// Metadata describing a binary target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BinaryMetadata {
    /// The binary source file.
    #[serde(flatten)]
    pub source: SourceFile,
    /// The target name used by Cargo.
    pub name: String,
}

/// Source entry points for a package's build script, library, and binaries.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetMetadata {
    /// The package build script.
    pub build_script: Option<SourceFile>,
    /// The package library target, if declared or inferred.
    pub library: Option<LibraryMetadata>,
    /// The package binary targets.
    pub binaries: Vec<BinaryMetadata>,
}

impl TargetMetadata {
    /// Completes a manifest and extracts metadata using its package path inventory.
    ///
    /// The inventory must contain package-relative paths. Completing the manifest
    /// mutates it to include inferred targets and defaults.
    pub fn extract<I, P>(manifest: &mut Manifest, paths: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let paths = paths
            .into_iter()
            .map(|path| normalize_path(path.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;

        let fs = PathsFileSystem::new(paths);
        manifest.complete_from_abstract_filesystem(&fs)?;

        let build_script = manifest
            .package
            .as_ref()
            .and_then(|package| match package.build.as_ref() {
                Some(StringOrBool::String(path)) => Some(path.as_str()),
                Some(StringOrBool::Bool(true)) => Some("build.rs"),
                Some(StringOrBool::Bool(false)) => None,
                None => None, // No build script was declared or discovered.
            })
            .map(|path| source_file(path, &fs))
            .transpose()?;

        let library = manifest
            .lib
            .as_ref()
            .map(|library| library_metadata(library, &fs))
            .transpose()?;

        let binaries = manifest
            .bin
            .iter()
            .map(|binary| binary_metadata(binary, &fs))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            build_script,
            library,
            binaries,
        })
    }

}

/// An error encountered while extracting package target metadata.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The manifest could not be completed from the file inventory.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// A target path is absolute or escapes the package root.
    #[error("target path `{0}` is outside the package root")]
    OutsidePackage(String),
    /// A target path resolves to the package root rather than a source file.
    #[error("target path `{0}` resolves to the package root")]
    PackageRoot(String),
    /// A completed target is missing a required field.
    #[error("{target} target is missing its `{field}` field")]
    MissingTargetField {
        /// A human-readable target identifier.
        target: String,
        /// The missing manifest field.
        field: &'static str,
    },
}

fn library_metadata(
    library: &Product,
    inventory: &PathsFileSystem,
) -> Result<LibraryMetadata, Error> {
    let name = target_field(library.name.as_deref(), "library", "name")?;
    let path = target_field(library.path.as_deref(), "library", "path")?;

    Ok(LibraryMetadata {
        source: source_file(path, inventory)?,
        name: name.to_string(),
        // Cargo also accepts `crate-type = ["proc-macro"]`, but warns against it.
        is_proc_macro: library.proc_macro
            || library
                .crate_type
                .iter()
                .flatten()
                .any(|crate_type| crate_type == "proc-macro"),
    })
}

fn binary_metadata(binary: &Product, inventory: &PathsFileSystem) -> Result<BinaryMetadata, Error> {
    let target = binary
        .name
        .as_deref()
        .map(|name| format!("binary `{name}`"))
        .unwrap_or_else(|| "binary".to_string());
    let name = target_field(binary.name.as_deref(), &target, "name")?;
    let path = target_field(binary.path.as_deref(), &target, "path")?;

    Ok(BinaryMetadata {
        source: source_file(path, inventory)?,
        name: name.to_string(),
    })
}

fn target_field<'a>(
    value: Option<&'a str>,
    target: &str,
    field_name: &'static str,
) -> Result<&'a str, Error> {
    value.ok_or_else(|| Error::MissingTargetField {
        target: target.to_string(),
        field: field_name,
    })
}

fn source_file(path: &str, inventory: &PathsFileSystem) -> Result<SourceFile, Error> {
    let normalized = normalize_path(Path::new(path))?;
    let exists = inventory.contains(&normalized);
    let path = normalized
        .into_os_string()
        .into_string()
        .expect("manifest paths are valid UTF-8");

    Ok(SourceFile { path, exists })
}

fn normalize_path(path: &Path) -> Result<PathBuf, Error> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => normalized.push(component),
            Component::ParentDir if normalized.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::OutsidePackage(path.display().to_string()));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        Err(Error::PackageRoot(path.display().to_string()))
    } else {
        Ok(normalized)
    }
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

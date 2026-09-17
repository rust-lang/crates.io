use std::collections::BTreeSet;
use std::fs::read_dir;
use std::io;
use std::path::{Component, Path, PathBuf};

/// A trait for abstracting over filesystem operations.
///
/// This trait is primarily used for target auto-discovery in the
/// [`complete_from_abstract_filesystem()`](crate::Manifest::complete_from_abstract_filesystem) method.
pub trait AbstractFilesystem {
    /// Returns a set of file and folder names in the given directory.
    ///
    /// This method should return a [`std::io::ErrorKind::NotFound`] error if the
    /// directory does not exist.
    fn file_names_in<T: AsRef<Path>>(&self, rel_path: T) -> io::Result<BTreeSet<Box<str>>>;
}

/// A [`AbstractFilesystem`] implementation that reads from the actual filesystem
/// within the given root path.
pub struct Filesystem<'a> {
    path: &'a Path,
}

impl<'a> Filesystem<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self { path }
    }
}

impl AbstractFilesystem for Filesystem<'_> {
    fn file_names_in<T: AsRef<Path>>(&self, rel_path: T) -> io::Result<BTreeSet<Box<str>>> {
        Ok(read_dir(self.path.join(rel_path))?
            .filter_map(|entry| {
                entry
                    .ok()
                    .map(|e| e.file_name().to_string_lossy().to_string().into_boxed_str())
            })
            .collect())
    }
}

/// An [`AbstractFilesystem`] backed by a list of package-relative paths.
pub struct PathsFileSystem(Vec<PathBuf>);

impl PathsFileSystem {
    /// Creates an abstract filesystem from package-relative paths.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self(paths)
    }

    /// Returns whether the backing path list contains the exact path.
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        self.0.iter().any(|candidate| candidate == path.as_ref())
    }
}

impl AbstractFilesystem for PathsFileSystem {
    fn file_names_in<T: AsRef<Path>>(&self, rel_path: T) -> io::Result<BTreeSet<Box<str>>> {
        let mut rel_path = rel_path.as_ref();

        // Deal with relative paths that start with `./`
        let mut components = rel_path.components();
        while components.next() == Some(Component::CurDir) {
            rel_path = components.as_path();
        }

        let file_names = self
            .0
            .iter()
            .filter_map(move |path| path.strip_prefix(rel_path).ok())
            .filter_map(|name| match name.components().next() {
                Some(Component::Normal(path)) => path.to_str(),
                _ => None,
            })
            .map(From::from)
            .collect();

        Ok(file_names)
    }
}

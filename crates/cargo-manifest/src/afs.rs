use std::collections::BTreeSet;
use std::fs::read_dir;
use std::io;
use std::path::Path;

/// A trait for abstracting over filesystem operations.
///
/// This trait is primarily used for target auto-discovery in the
/// [`complete_from_abstract_filesystem()`](crate::Manifest::complete_from_abstract_filesystem) method.
pub trait AbstractFilesystem {
    /// Returns a set of file and folder names in the given directory.
    ///
    /// This method should return a [std::io::ErrorKind::NotFound] error if the
    /// directory does not exist.
    fn file_names_in<T: AsRef<Path>>(&self, rel_path: T) -> io::Result<BTreeSet<Box<str>>>;
}

/// A [AbstractFilesystem] implementation that reads from the actual filesystem
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

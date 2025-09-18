use crate::languages::should_ignore_language;
use std::path::Path;
use tokei::LanguageType;

#[derive(Debug, Clone, Copy)]
pub struct PathDetails {
    is_benchmark: bool,
    is_example: bool,
    is_hidden: bool,
    is_test: bool,
    language_type: Option<LanguageType>,
}

impl PathDetails {
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_lowercase();

        let is_benchmark = path_str.contains("benches/") || path_str.contains("benchmark/");
        let is_example = path_str.contains("examples/");
        let is_test = path_str.contains("tests/")
            || path_str.contains("test/")
            || path_str.contains("testing/");

        let is_hidden = path
            .file_name()
            .map(|filename| filename.to_string_lossy().starts_with('.'))
            .unwrap_or(false);

        let language_type = path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(LanguageType::from_file_extension);

        Self {
            is_benchmark,
            is_example,
            is_hidden,
            is_test,
            language_type,
        }
    }

    /// Determine if the file should be ignored for line counting purposes
    /// because it is a benchmark, example, hidden, or test file.
    pub fn should_ignore(&self) -> bool {
        self.is_benchmark || self.is_example || self.is_hidden || self.is_test
    }

    /// Get the actual detected language type, even if it should be ignored.
    pub fn actual_language_type(&self) -> Option<LanguageType> {
        self.language_type
    }

    /// Get the detected language type, returning `None` if no language was
    /// detected or if the language should be ignored (e.g., data files).
    pub fn language_type(&self) -> Option<LanguageType> {
        self.language_type.filter(|lt| !should_ignore_language(*lt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_should_count_path() {
        assert_debug_snapshot!(PathDetails::from_path(Path::new("src/tests/mod.rs")));
        assert_debug_snapshot!(PathDetails::from_path(Path::new("tests/integration.rs")));
        assert_debug_snapshot!(PathDetails::from_path(Path::new("examples/basic.rs")));
        assert_debug_snapshot!(PathDetails::from_path(Path::new("benches/bench.rs")));
        assert_debug_snapshot!(PathDetails::from_path(Path::new("src/lib.rs")));
    }
}

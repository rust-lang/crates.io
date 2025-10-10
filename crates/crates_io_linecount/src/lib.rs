mod languages;
mod paths;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokei::Config;

pub use crate::paths::PathDetails;

// Re-export LanguageType for use by other crates
pub use tokei::LanguageType;

/// Tokei configuration used for analysis (cached)
static TOKEI_CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    no_ignore: Some(true),
    treat_doc_strings_as_comments: Some(true),
    ..Default::default()
});

/// Statistics for a single programming language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LanguageStats {
    /// Number of lines of code (excluding comments and blank lines)
    pub code_lines: usize,
    /// Number of comment lines
    pub comment_lines: usize,
    /// Number of files of this language
    pub files: usize,
}

/// Complete line count statistics for a crate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LinecountStats {
    /// Per-language breakdown of line counts
    pub languages: HashMap<LanguageType, LanguageStats>,
    /// Total lines of code across all languages
    pub total_code_lines: usize,
    /// Total comment lines across all languages
    pub total_comment_lines: usize,
}

impl LinecountStats {
    /// Create a new empty statistics collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single file to the statistics
    ///
    /// The caller can use `should_count_path()` to check if a file should be processed
    /// before decompressing to avoid unnecessary work.
    pub fn add_file(&mut self, language_type: LanguageType, content: &[u8]) {
        let file_stats = language_type.parse_from_slice(content, &TOKEI_CONFIG);

        // Update language-specific stats
        let entry = self.languages.entry(language_type).or_default();
        entry.code_lines += file_stats.code;
        entry.comment_lines += file_stats.comments;
        entry.files += 1;

        // Update totals
        self.total_code_lines += file_stats.code;
        self.total_comment_lines += file_stats.comments;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_empty() {
        let stats = LinecountStats::new();
        insta::assert_json_snapshot!(stats, @r#"
        {
          "languages": {},
          "total_code_lines": 0,
          "total_comment_lines": 0
        }
        "#);
    }

    #[test]
    fn test_add_file() {
        let mut stats = LinecountStats::new();

        // Add a Rust file
        let rust_code = b"// This is a comment\nfn main() {\n    println!(\"Hello\");\n}";
        stats.add_file(LanguageType::Rust, rust_code);

        insta::assert_json_snapshot!(stats, @r#"
        {
          "languages": {
            "Rust": {
              "code_lines": 3,
              "comment_lines": 1,
              "files": 1
            }
          },
          "total_code_lines": 3,
          "total_comment_lines": 1
        }
        "#);
    }

    #[test]
    fn test_workflow() {
        let mut stats = LinecountStats::new();

        let files = [
            ("src/lib.rs", "pub fn hello() {}"),
            ("tests/test.rs", "fn test() {}"), // Should be skipped
            ("README.md", "# Hello"),          // Should be skipped
        ];

        for (path, content) in files {
            let path = Path::new(path);
            let path_details = PathDetails::from_path(path);

            if !path_details.should_ignore()
                && let Some(language_type) = path_details.language_type()
            {
                stats.add_file(language_type, content.as_bytes())
            };
        }

        insta::assert_json_snapshot!(stats, @r#"
        {
          "languages": {
            "Rust": {
              "code_lines": 1,
              "comment_lines": 0,
              "files": 1
            }
          },
          "total_code_lines": 1,
          "total_comment_lines": 0
        }
        "#);
    }
}

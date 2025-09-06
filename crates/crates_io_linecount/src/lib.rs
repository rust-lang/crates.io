use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use tokei::Config;

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

/// Check if a path should be counted and return its language type
///
/// Returns `Some(LanguageType)` if the file should be analyzed, `None` otherwise.
pub fn should_count_path(path: &Path) -> Option<LanguageType> {
    let path_str = path.to_string_lossy().to_lowercase();

    // Skip test and example directories
    if path_str.contains("tests/")
        || path_str.contains("test/")
        || path_str.contains("testing/")
        || path_str.contains("examples/")
        || path_str.contains("benches/")
        || path_str.contains("benchmark/")
    {
        return None;
    }

    // Skip hidden files
    if let Some(filename) = path.file_name() {
        if filename.to_string_lossy().starts_with('.') {
            return None;
        }
    }

    // Get language type from file extension
    let extension = path.extension().and_then(|ext| ext.to_str())?;
    let language_type = LanguageType::from_file_extension(extension)?;

    // Only count if it's a programming language
    is_countable_language(language_type).then_some(language_type)
}

/// Determine if a language should be counted
fn is_countable_language(lang: LanguageType) -> bool {
    !matches!(
        lang,
        // Configuration and data files
        LanguageType::Json |
        LanguageType::Yaml |
        LanguageType::Toml |
        LanguageType::Xml |
        LanguageType::Ini |

        // Documentation
        LanguageType::Markdown |
        LanguageType::Text |
        LanguageType::ReStructuredText |
        LanguageType::AsciiDoc |
        LanguageType::Org |

        // Build system files
        LanguageType::Makefile |
        LanguageType::CMake |
        LanguageType::Dockerfile |
        LanguageType::Autoconf |
        LanguageType::MsBuild |
        LanguageType::Meson |
        LanguageType::Scons |
        LanguageType::Bazel |
        LanguageType::Nix |

        // Shell scripts (debatable, but often just build/deploy automation)
        LanguageType::Batch |
        LanguageType::PowerShell |

        // Other non-programming files
        LanguageType::Svg |
        LanguageType::Hex |
        LanguageType::Protobuf |
        LanguageType::Thrift
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};

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
            if let Some(language_type) = should_count_path(path) {
                stats.add_file(language_type, content.as_bytes());
            }
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

    #[test]
    fn test_should_count_path() {
        assert_none!(should_count_path(Path::new("src/tests/mod.rs")));
        assert_none!(should_count_path(Path::new("tests/integration.rs")));
        assert_none!(should_count_path(Path::new("examples/basic.rs")));
        assert_none!(should_count_path(Path::new("benches/bench.rs")));
        assert_some!(should_count_path(Path::new("src/lib.rs")));
    }

    #[test]
    fn test_language_filtering() {
        // Should count programming languages
        assert!(is_countable_language(LanguageType::Rust));
        assert!(is_countable_language(LanguageType::JavaScript));
        assert!(is_countable_language(LanguageType::Html));
        assert!(is_countable_language(LanguageType::Css));

        // Should skip config/data files
        assert!(!is_countable_language(LanguageType::Json));
        assert!(!is_countable_language(LanguageType::Yaml));
        assert!(!is_countable_language(LanguageType::Toml));
        assert!(!is_countable_language(LanguageType::Markdown));
    }
}

use tokei::LanguageType;

/// Determine if a language should be counted or ignored
pub fn should_ignore_language(lang: LanguageType) -> bool {
    matches!(
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

    #[test]
    fn test_should_ignore_language() {
        // Should count programming languages
        assert!(!should_ignore_language(LanguageType::Rust));
        assert!(!should_ignore_language(LanguageType::JavaScript));
        assert!(!should_ignore_language(LanguageType::Html));
        assert!(!should_ignore_language(LanguageType::Css));

        // Should skip config/data files
        assert!(should_ignore_language(LanguageType::Json));
        assert!(should_ignore_language(LanguageType::Yaml));
        assert!(should_ignore_language(LanguageType::Toml));
        assert!(should_ignore_language(LanguageType::Markdown));
    }
}

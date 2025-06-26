//! Error types for the crates_io_og_image crate.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when generating OpenGraph images.
#[derive(Debug, Error)]
pub enum OgImageError {
    /// Failed to find or execute the Typst binary.
    #[error("Failed to find or execute Typst binary: {0}")]
    TypstNotFound(#[source] std::io::Error),

    /// Environment variable error.
    #[error("Environment variable error: {0}")]
    EnvVarError(anyhow::Error),

    /// Failed to download avatar from URL.
    #[error("Failed to download avatar from URL '{url}': {source}")]
    AvatarDownloadError {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// Failed to write avatar to file.
    #[error("Failed to write avatar to file at {path:?}: {source}")]
    AvatarWriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Template rendering error.
    #[error("Template rendering error: {0}")]
    TemplateError(#[from] minijinja::Error),

    /// Typst compilation failed.
    #[error("Typst compilation failed: {stderr}")]
    TypstCompilationError {
        stderr: String,
        stdout: String,
        exit_code: Option<i32>,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Temporary file creation error.
    #[error("Failed to create temporary file: {0}")]
    TempFileError(std::io::Error),

    /// Temporary directory creation error.
    #[error("Failed to create temporary directory: {0}")]
    TempDirError(std::io::Error),
}

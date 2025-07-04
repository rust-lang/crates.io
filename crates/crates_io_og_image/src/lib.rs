#![doc = include_str!("../README.md")]

mod error;
mod formatting;

pub use error::OgImageError;

use crate::formatting::{serialize_bytes, serialize_number, serialize_optional_number};
use crates_io_env_vars::var;
use reqwest::StatusCode;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, error, info, instrument, warn};

/// Data structure containing information needed to generate an OpenGraph image
/// for a crates.io crate.
#[derive(Debug, Clone, Serialize)]
pub struct OgImageData<'a> {
    /// The crate name
    pub name: &'a str,
    /// Latest version string (e.g., "1.0.210")
    pub version: &'a str,
    /// Crate description text
    pub description: Option<&'a str>,
    /// License information (e.g., "MIT/Apache-2.0")
    pub license: Option<&'a str>,
    /// Keywords/categories for the crate
    pub tags: &'a [&'a str],
    /// Author information
    pub authors: &'a [OgImageAuthorData<'a>],
    /// Source lines of code count (optional)
    #[serde(serialize_with = "serialize_optional_number")]
    pub lines_of_code: Option<u32>,
    /// Package size in bytes
    #[serde(serialize_with = "serialize_bytes")]
    pub crate_size: u32,
    /// Total number of releases
    #[serde(serialize_with = "serialize_number")]
    pub releases: u32,
}

/// Author information for OpenGraph image generation
#[derive(Debug, Clone, Serialize)]
pub struct OgImageAuthorData<'a> {
    /// Author username/name
    pub name: &'a str,
    /// Optional avatar URL
    pub avatar: Option<Cow<'a, str>>,
}

impl<'a> OgImageAuthorData<'a> {
    /// Creates a new `OgImageAuthorData` with the specified name and optional avatar.
    pub const fn new(name: &'a str, avatar: Option<Cow<'a, str>>) -> Self {
        Self { name, avatar }
    }

    /// Creates a new `OgImageAuthorData` with a URL-based avatar.
    pub fn with_url(name: &'a str, url: impl Into<Cow<'a, str>>) -> Self {
        Self::new(name, Some(url.into()))
    }
}

/// Generator for creating OpenGraph images using the Typst typesetting system.
///
/// This struct manages the path to the Typst binary and provides methods for
/// generating PNG images from a Typst template.
pub struct OgImageGenerator {
    typst_binary_path: PathBuf,
    typst_font_path: Option<PathBuf>,
    oxipng_binary_path: PathBuf,
}

impl OgImageGenerator {
    /// Creates a new `OgImageGenerator` with default binary paths.
    ///
    /// Uses "typst" and "oxipng" as default binary paths, assuming they are
    /// available in PATH. Use [`with_typst_path()`](Self::with_typst_path) and
    /// [`with_oxipng_path()`](Self::with_oxipng_path) to customize the
    /// binary paths.
    ///
    /// # Examples
    ///
    /// ```
    /// use crates_io_og_image::OgImageGenerator;
    ///
    /// let generator = OgImageGenerator::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Detects the image format from the first few bytes using magic numbers.
    ///
    /// Returns the appropriate file extension for supported formats:
    /// - PNG: returns "png"
    /// - JPEG: returns "jpg"
    /// - Unsupported formats: returns None
    fn detect_image_format(bytes: &[u8]) -> Option<&'static str> {
        // PNG magic number: 89 50 4E 47 0D 0A 1A 0A
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some("png");
        }

        // JPEG magic number: FF D8 FF
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some("jpg");
        }

        None
    }

    /// Creates a new `OgImageGenerator` using the `TYPST_PATH` environment variable.
    ///
    /// If the `TYPST_PATH` environment variable is set, uses that path.
    /// Otherwise, falls back to the default behavior (assumes "typst" is in PATH).
    ///
    /// # Examples
    ///
    /// ```
    /// use crates_io_og_image::OgImageGenerator;
    ///
    /// let generator = OgImageGenerator::from_environment()?;
    /// # Ok::<(), crates_io_og_image::OgImageError>(())
    /// ```
    #[instrument]
    pub fn from_environment() -> Result<Self, OgImageError> {
        let typst_path = var("TYPST_PATH").map_err(OgImageError::EnvVarError)?;
        let font_path = var("TYPST_FONT_PATH").map_err(OgImageError::EnvVarError)?;
        let oxipng_path = var("OXIPNG_PATH").map_err(OgImageError::EnvVarError)?;

        let mut generator = OgImageGenerator::default();

        if let Some(ref path) = typst_path {
            debug!(typst_path = %path, "Using custom Typst binary path from environment");
            generator.typst_binary_path = PathBuf::from(path);
        } else {
            debug!("Using default Typst binary path (assumes 'typst' in PATH)");
        };

        if let Some(ref font_path) = font_path {
            debug!(font_path = %font_path, "Setting custom font path from environment");
            generator.typst_font_path = Some(PathBuf::from(font_path));
        } else {
            debug!("No custom font path specified, using Typst default font discovery");
        }

        if let Some(ref path) = oxipng_path {
            debug!(oxipng_path = %path, "Using custom oxipng binary path from environment");
            generator.oxipng_binary_path = PathBuf::from(path);
        } else {
            debug!("OXIPNG_PATH not set, defaulting to 'oxipng' in PATH");
        };

        Ok(generator)
    }

    /// Sets the Typst binary path for the generator.
    ///
    /// This allows specifying a custom path to the Typst binary.
    /// If not set, defaults to "typst" which assumes the binary is available in PATH.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use crates_io_og_image::OgImageGenerator;
    ///
    /// let generator = OgImageGenerator::default()
    ///     .with_typst_path(PathBuf::from("/usr/local/bin/typst"));
    /// ```
    pub fn with_typst_path(mut self, typst_path: PathBuf) -> Self {
        self.typst_binary_path = typst_path;
        self
    }

    /// Sets the font path for the Typst compiler.
    ///
    /// This allows specifying a custom directory where Typst will look for fonts
    /// during compilation. Setting a custom font directory implies using the
    /// `--ignore-system-fonts` flag of the Typst CLI. If not set, Typst will
    /// use its default font discovery.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use crates_io_og_image::OgImageGenerator;
    ///
    /// let generator = OgImageGenerator::default()
    ///     .with_font_path(PathBuf::from("/usr/share/fonts"));
    /// ```
    pub fn with_font_path(mut self, font_path: PathBuf) -> Self {
        self.typst_font_path = Some(font_path);
        self
    }

    /// Sets the oxipng binary path for PNG optimization.
    ///
    /// This allows specifying a custom path to the oxipng binary for PNG optimization.
    /// If not set, defaults to "oxipng" which assumes the binary is available in PATH.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use crates_io_og_image::OgImageGenerator;
    ///
    /// let generator = OgImageGenerator::default()
    ///     .with_oxipng_path(PathBuf::from("/usr/local/bin/oxipng"));
    /// ```
    pub fn with_oxipng_path(mut self, oxipng_path: PathBuf) -> Self {
        self.oxipng_binary_path = oxipng_path;
        self
    }

    /// Processes avatars by downloading URLs and copying assets to the assets directory.
    ///
    /// This method handles both asset-based avatars (which are copied from the bundled assets)
    /// and URL-based avatars (which are downloaded from the internet).
    /// Returns a mapping from avatar source to the local filename.
    #[instrument(skip(self, data), fields(crate.name = %data.name))]
    async fn process_avatars<'a>(
        &self,
        data: &'a OgImageData<'_>,
        assets_dir: &Path,
    ) -> Result<HashMap<&'a str, String>, OgImageError> {
        let mut avatar_map = HashMap::new();

        let client = reqwest::Client::new();
        for (index, author) in data.authors.iter().enumerate() {
            if let Some(avatar) = &author.avatar {
                debug!(
                    author_name = %author.name,
                    avatar_url = %avatar,
                    "Processing avatar for author {}", author.name
                );

                // Download the avatar from the URL
                debug!(url = %avatar, "Downloading avatar from URL: {avatar}");
                let response = client.get(avatar.as_ref()).send().await.map_err(|err| {
                    OgImageError::AvatarDownloadError {
                        url: avatar.to_string(),
                        source: err,
                    }
                })?;

                let status = response.status();
                if status == StatusCode::NOT_FOUND {
                    warn!(url = %avatar, "Avatar URL returned 404 Not Found");
                    continue; // Skip this avatar if not found
                }

                if let Err(err) = response.error_for_status_ref() {
                    return Err(OgImageError::AvatarDownloadError {
                        url: avatar.to_string(),
                        source: err,
                    });
                }

                let content_length = response.content_length();
                debug!(
                    url = %avatar,
                    content_length = ?content_length,
                    status = %response.status(),
                    "Avatar download response received"
                );

                let bytes = response.bytes().await;
                let bytes = bytes.map_err(|err| {
                    error!(url = %avatar, error = %err, "Failed to read avatar response bytes");
                    OgImageError::AvatarDownloadError {
                        url: (*avatar).to_string(),
                        source: err,
                    }
                })?;

                debug!(url = %avatar, size_bytes = bytes.len(), "Avatar downloaded successfully");

                // Detect the image format and determine the appropriate file extension
                let Some(extension) = Self::detect_image_format(&bytes) else {
                    // Format not supported, log warning with first 20 bytes for debugging
                    let debug_bytes = &bytes[..bytes.len().min(20)];
                    let hex_bytes = debug_bytes
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ");

                    warn!("Unsupported avatar format at {avatar}, first 20 bytes: {hex_bytes}");

                    // Skip this avatar and continue with the next one
                    continue;
                };

                let filename = format!("avatar_{index}.{extension}");
                let avatar_path = assets_dir.join(&filename);

                debug!(
                    author_name = %author.name,
                    avatar_url = %avatar,
                    avatar_path = %avatar_path.display(),
                    "Writing avatar file with detected format"
                );

                // Write the bytes to the avatar file
                fs::write(&avatar_path, &bytes).await.map_err(|err| {
                    OgImageError::AvatarWriteError {
                        path: avatar_path.clone(),
                        source: err,
                    }
                })?;

                debug!(
                    author_name = %author.name,
                    path = %avatar_path.display(),
                    size_bytes = bytes.len(),
                    "Avatar processed and written successfully"
                );

                // Store the mapping from the avatar source to the numbered filename
                avatar_map.insert(avatar.as_ref(), filename);
            }
        }

        Ok(avatar_map)
    }

    /// Generates an OpenGraph image using the provided data.
    ///
    /// This method creates a temporary directory with all the necessary files
    /// to create the OpenGraph image, compiles it to PNG using the Typst
    /// binary, and returns the resulting image as a `NamedTempFile`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crates_io_og_image::{OgImageGenerator, OgImageData, OgImageAuthorData, OgImageError};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), OgImageError> {
    /// let generator = OgImageGenerator::default();
    /// let data = OgImageData {
    ///     name: "my-crate",
    ///     version: "1.0.0",
    ///     description: Some("A sample crate"),
    ///     license: Some("MIT"),
    ///     tags: &["web", "api"],
    ///     authors: &[OgImageAuthorData { name: "user", avatar: None }],
    ///     lines_of_code: Some(5000),
    ///     crate_size: 100,
    ///     releases: 10,
    /// };
    /// let image_file = generator.generate(data).await?;
    /// println!("Generated image at: {:?}", image_file.path());
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, data), fields(
        crate.name = %data.name,
        crate.version = %data.version,
        author_count = data.authors.len(),
    ))]
    pub async fn generate(&self, data: OgImageData<'_>) -> Result<NamedTempFile, OgImageError> {
        let start_time = std::time::Instant::now();
        info!("Starting OpenGraph image generation");

        // Create a temporary folder
        let temp_dir = tempfile::tempdir().map_err(OgImageError::TempDirError)?;
        debug!(temp_dir = %temp_dir.path().display(), "Created temporary directory");

        // Create assets directory and copy logo and icons
        let assets_dir = temp_dir.path().join("assets");
        debug!(assets_dir = %assets_dir.display(), "Creating assets directory");
        fs::create_dir(&assets_dir).await?;

        debug!("Copying bundled assets to temporary directory");
        let cargo_logo = include_bytes!("../template/assets/cargo.png");
        fs::write(assets_dir.join("cargo.png"), cargo_logo).await?;
        let rust_logo_svg = include_bytes!("../template/assets/rust-logo.svg");
        fs::write(assets_dir.join("rust-logo.svg"), rust_logo_svg).await?;

        // Copy SVG icons
        debug!("Copying SVG icon assets");
        let code_branch_svg = include_bytes!("../template/assets/code-branch.svg");
        fs::write(assets_dir.join("code-branch.svg"), code_branch_svg).await?;
        let code_svg = include_bytes!("../template/assets/code.svg");
        fs::write(assets_dir.join("code.svg"), code_svg).await?;
        let scale_balanced_svg = include_bytes!("../template/assets/scale-balanced.svg");
        fs::write(assets_dir.join("scale-balanced.svg"), scale_balanced_svg).await?;
        let tag_svg = include_bytes!("../template/assets/tag.svg");
        fs::write(assets_dir.join("tag.svg"), tag_svg).await?;
        let weight_hanging_svg = include_bytes!("../template/assets/weight-hanging.svg");
        fs::write(assets_dir.join("weight-hanging.svg"), weight_hanging_svg).await?;

        // Process avatars - download URLs and copy assets
        let avatar_start_time = std::time::Instant::now();
        info!("Processing avatars");
        let avatar_map = self.process_avatars(&data, &assets_dir).await?;
        let avatar_duration = avatar_start_time.elapsed();
        info!(
            avatar_count = avatar_map.len(),
            duration_ms = avatar_duration.as_millis(),
            "Avatar processing completed"
        );

        // Copy the static Typst template file
        let template_content = include_str!("../template/og-image.typ");
        let typ_file_path = temp_dir.path().join("og-image.typ");
        debug!(template_path = %typ_file_path.display(), "Copying Typst template");
        fs::write(&typ_file_path, template_content).await?;

        // Create a named temp file for the output PNG
        let output_file = NamedTempFile::new().map_err(OgImageError::TempFileError)?;
        debug!(output_path = %output_file.path().display(), "Created output file");

        // Serialize data and avatar_map to JSON
        debug!("Serializing data and avatar map to JSON");
        let json_data =
            serde_json::to_string(&data).map_err(OgImageError::JsonSerializationError)?;

        let json_avatar_map =
            serde_json::to_string(&avatar_map).map_err(OgImageError::JsonSerializationError)?;

        // Run typst compile command with input data
        info!("Running Typst compilation command");
        let mut command = Command::new(&self.typst_binary_path);
        command.arg("compile").arg("--format").arg("png");

        // Pass in the data and avatar map as JSON inputs
        let input = format!("data={json_data}");
        command.arg("--input").arg(input);
        let input = format!("avatar_map={json_avatar_map}");
        command.arg("--input").arg(input);

        // Pass in the font path if specified
        if let Some(font_path) = &self.typst_font_path {
            debug!(font_path = %font_path.display(), "Using custom font path");
            command.arg("--font-path").arg(font_path);
            command.arg("--ignore-system-fonts");
        } else {
            debug!("Using system font discovery");
        }

        // Pass input and output file paths
        command.arg(&typ_file_path).arg(output_file.path());

        // Clear environment variables to avoid leaking sensitive data
        command.env_clear();

        // Preserve environment variables needed for font discovery
        if let Ok(path) = std::env::var("PATH") {
            command.env("PATH", path);
        }
        if let Ok(home) = std::env::var("HOME") {
            command.env("HOME", home);
        }

        let compilation_start_time = std::time::Instant::now();
        let output = command.output().await;
        let output = output.map_err(OgImageError::TypstNotFound)?;
        let compilation_duration = compilation_start_time.elapsed();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            error!(
                exit_code = ?output.status.code(),
                stderr = %stderr,
                stdout = %stdout,
                duration_ms = compilation_duration.as_millis(),
                "Typst compilation failed"
            );
            return Err(OgImageError::TypstCompilationError {
                stderr,
                stdout,
                exit_code: output.status.code(),
            });
        }

        let output_size_bytes = fs::metadata(output_file.path()).await;
        let output_size_bytes = output_size_bytes.map(|m| m.len()).unwrap_or(0);

        debug!(
            duration_ms = compilation_duration.as_millis(),
            output_size_bytes, "Typst compilation completed successfully"
        );

        // After successful Typst compilation, optimize the PNG
        self.optimize_png(output_file.path()).await;

        let duration = start_time.elapsed();
        info!(
            duration_ms = duration.as_millis(),
            output_size_bytes, "OpenGraph image generation completed successfully"
        );
        Ok(output_file)
    }

    /// Optimizes a PNG file using oxipng.
    ///
    /// This method attempts to reduce the file size of a PNG using lossless compression.
    /// All errors are handled internally and logged as warnings. The method never fails
    /// to ensure PNG optimization is truly optional.
    async fn optimize_png(&self, png_file: &Path) {
        debug!(
            input_file = %png_file.display(),
            oxipng_path = %self.oxipng_binary_path.display(),
            "Starting PNG optimization"
        );

        let start_time = std::time::Instant::now();

        let mut command = Command::new(&self.oxipng_binary_path);

        // Default optimization level for speed/compression balance
        command.arg("--opt").arg("2");

        // Remove safe-to-remove metadata
        command.arg("--strip").arg("safe");

        // Overwrite the input PNG file
        command.arg(png_file);

        // Clear environment variables to avoid leaking sensitive data
        command.env_clear();

        // Preserve environment variables needed for running oxipng
        if let Ok(path) = std::env::var("PATH") {
            command.env("PATH", path);
        }

        let output = command.output().await;
        let duration = start_time.elapsed();

        match output {
            Ok(output) if output.status.success() => {
                debug!(
                    duration_ms = duration.as_millis(),
                    "PNG optimization completed successfully"
                );
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                warn!(
                    exit_code = ?output.status.code(),
                    stderr = %stderr,
                    stdout = %stdout,
                    duration_ms = duration.as_millis(),
                    input_file = %png_file.display(),
                    "PNG optimization failed, continuing with unoptimized image"
                );
            }
            Err(err) => {
                warn!(
                    error = %err,
                    input_file = %png_file.display(),
                    oxipng_path = %self.oxipng_binary_path.display(),
                    "Failed to execute oxipng, continuing with unoptimized image"
                );
            }
        }
    }
}

impl Default for OgImageGenerator {
    /// Creates a default `OgImageGenerator` with default binary paths.
    ///
    /// Uses "typst" and "oxipng" as default binary paths, assuming they are available in PATH.
    fn default() -> Self {
        Self {
            typst_binary_path: PathBuf::from("typst"),
            typst_font_path: None,
            oxipng_binary_path: PathBuf::from("oxipng"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Server, ServerGuard};
    use tracing::dispatcher::DefaultGuard;
    use tracing::{Level, subscriber};
    use tracing_subscriber::fmt;

    fn init_tracing() -> DefaultGuard {
        let subscriber = fmt()
            .compact()
            .with_max_level(Level::DEBUG)
            .with_test_writer()
            .finish();

        subscriber::set_default(subscriber)
    }

    async fn create_mock_avatar_server() -> ServerGuard {
        let mut server = Server::new_async().await;

        // Mock for successful avatar download
        server
            .mock("GET", "/test-avatar.png")
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body(include_bytes!("../template/assets/test-avatar.png"))
            .create();

        // Mock for 404 avatar download
        server
            .mock("GET", "/missing-avatar.png")
            .with_status(404)
            .with_header("content-type", "text/plain")
            .with_body("Not Found")
            .create();

        server
    }

    const fn author(name: &str) -> OgImageAuthorData<'_> {
        OgImageAuthorData::new(name, None)
    }

    fn author_with_avatar(name: &str, url: String) -> OgImageAuthorData<'_> {
        OgImageAuthorData::with_url(name, url)
    }

    fn create_minimal_test_data() -> OgImageData<'static> {
        static AUTHORS: &[OgImageAuthorData<'_>] = &[author("author")];

        OgImageData {
            name: "minimal-crate",
            version: "1.0.0",
            description: None,
            license: None,
            tags: &[],
            authors: AUTHORS,
            lines_of_code: None,
            crate_size: 10000,
            releases: 1,
        }
    }

    fn create_escaping_authors(server_url: &str) -> Vec<OgImageAuthorData<'_>> {
        vec![
            author_with_avatar(
                "author \"with quotes\"",
                format!("{server_url}/test-avatar.png"),
            ),
            author("author\\with\\backslashes"),
            author("author#with#hashes"),
        ]
    }

    fn create_escaping_test_data<'a>(authors: &'a [OgImageAuthorData<'a>]) -> OgImageData<'a> {
        OgImageData {
            name: "crate-with-\"quotes\"",
            version: "1.0.0-\"beta\"",
            description: Some(
                "A crate with \"quotes\", \\ backslashes, and other special chars: #[]{}()",
            ),
            license: Some("MIT OR \"Apache-2.0\""),
            tags: &[
                "tag-with-\"quotes\"",
                "tag\\with\\backslashes",
                "tag#with#symbols",
            ],
            authors,
            lines_of_code: Some(42),
            crate_size: 256256,
            releases: 5,
        }
    }

    fn create_overflow_authors(server_url: &str) -> Vec<OgImageAuthorData<'_>> {
        let avatar_url = format!("{server_url}/test-avatar.png");
        vec![
            author_with_avatar("alice-wonderland", avatar_url.clone()),
            author("bob-the-builder"),
            author_with_avatar("charlie-brown", avatar_url.clone()),
            author("diana-prince"),
            author_with_avatar("edward-scissorhands", avatar_url.clone()),
            author("fiona-apple"),
            author("george-washington"),
            author_with_avatar("helen-keller", avatar_url.clone()),
            author("isaac-newton"),
            author("jane-doe"),
        ]
    }

    fn create_overflow_test_data<'a>(authors: &'a [OgImageAuthorData<'a>]) -> OgImageData<'a> {
        OgImageData {
            name: "super-long-crate-name-for-testing-overflow-behavior",
            version: "2.1.0-beta.1+build.12345",
            description: Some(
                "This is an extremely long description that tests how the layout handles descriptions that might wrap to multiple lines or overflow the available space in the OpenGraph image template design. This is an extremely long description that tests how the layout handles descriptions that might wrap to multiple lines or overflow the available space in the OpenGraph image template design.",
            ),
            license: Some("MIT/Apache-2.0/ISC/BSD-3-Clause"),
            tags: &[
                "web-framework",
                "async-runtime",
                "database-orm",
                "serialization",
                "networking",
            ],
            authors,
            lines_of_code: Some(147000),
            crate_size: 2847123,
            releases: 1432,
        }
    }

    fn create_simple_test_data() -> OgImageData<'static> {
        static AUTHORS: &[OgImageAuthorData<'_>] = &[author("test-user")];

        OgImageData {
            name: "test-crate",
            version: "1.0.0",
            description: Some("A test crate for OpenGraph image generation"),
            license: Some("MIT/Apache-2.0"),
            tags: &["testing", "og-image"],
            authors: AUTHORS,
            lines_of_code: Some(1000),
            crate_size: 42012,
            releases: 1,
        }
    }

    fn skip_if_typst_unavailable() -> bool {
        if matches!(var("CI"), Ok(Some(_))) {
            // Do not skip tests in CI environments, even if Typst is unavailable.
            // We want the test to fail instead of silently skipping.
            return false;
        }

        std::process::Command::new("typst")
            .arg("--version")
            .output()
            .inspect_err(|_| {
                eprintln!("Skipping test: typst binary not found in PATH");
            })
            .is_err()
    }

    async fn generate_image(data: OgImageData<'_>) -> Option<Vec<u8>> {
        if skip_if_typst_unavailable() {
            return None;
        }

        let generator =
            OgImageGenerator::from_environment().expect("Failed to create OgImageGenerator");

        let temp_file = generator
            .generate(data)
            .await
            .expect("Failed to generate image");

        Some(std::fs::read(temp_file.path()).expect("Failed to read generated image"))
    }

    #[tokio::test]
    async fn test_generate_og_image_snapshot() {
        let _guard = init_tracing();
        let data = create_simple_test_data();

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_overflow_snapshot() {
        let _guard = init_tracing();

        let server = create_mock_avatar_server().await;
        let server_url = server.url();

        let authors = create_overflow_authors(&server_url);
        let data = create_overflow_test_data(&authors);

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image_overflow.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_minimal_snapshot() {
        let _guard = init_tracing();
        let data = create_minimal_test_data();

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image_minimal.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_escaping_snapshot() {
        let _guard = init_tracing();

        let server = create_mock_avatar_server().await;
        let server_url = server.url();

        let authors = create_escaping_authors(&server_url);
        let data = create_escaping_test_data(&authors);

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image_escaping.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_with_404_avatar() {
        let _guard = init_tracing();

        let server = create_mock_avatar_server().await;
        let server_url = server.url();

        // Create test data with a 404 avatar URL - should skip the avatar gracefully
        let authors = vec![author_with_avatar(
            "test-user",
            format!("{server_url}/missing-avatar.png"),
        )];
        let data = OgImageData {
            name: "test-crate-404",
            version: "1.0.0",
            description: Some("A test crate with 404 avatar"),
            license: Some("MIT"),
            tags: &["testing"],
            authors: &authors,
            lines_of_code: Some(1000),
            crate_size: 42012,
            releases: 1,
        };

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("404-avatar.png", image_data);
        }
    }
}

#![doc = include_str!("../README.md")]

mod error;
mod formatting;

pub use error::OgImageError;

use crate::formatting::{serialize_bytes, serialize_number, serialize_optional_number};
use bytes::Bytes;
use crates_io_env_vars::var;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::process::Command;

/// Data structure containing information needed to generate an OpenGraph image
/// for a crates.io crate.
#[derive(Debug, Clone, Serialize)]
pub struct OgImageData<'a> {
    /// The crate name
    pub name: &'a str,
    /// Latest version string (e.g., "v1.0.210")
    pub version: &'a str,
    /// Crate description text
    pub description: &'a str,
    /// License information (e.g., "MIT/Apache-2.0")
    pub license: &'a str,
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
    /// Optional avatar - either "test-avatar" for the test avatar or a URL
    pub avatar: Option<&'a str>,
}

impl<'a> OgImageAuthorData<'a> {
    /// Creates a new `OgImageAuthorData` with the specified name and optional avatar.
    pub const fn new(name: &'a str, avatar: Option<&'a str>) -> Self {
        Self { name, avatar }
    }

    /// Creates a new `OgImageAuthorData` with a URL-based avatar.
    pub fn with_url(name: &'a str, url: &'a str) -> Self {
        Self::new(name, Some(url))
    }

    /// Creates a new `OgImageAuthorData` with the test avatar.
    pub fn with_test_avatar(name: &'a str) -> Self {
        Self::with_url(name, "test-avatar")
    }
}

/// Generator for creating OpenGraph images using the Typst typesetting system.
///
/// This struct manages the path to the Typst binary and provides methods for
/// generating PNG images from a Typst template.
pub struct OgImageGenerator {
    typst_binary_path: PathBuf,
}

impl OgImageGenerator {
    /// Creates a new `OgImageGenerator` with the specified path to the Typst binary.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use crates_io_og_image::OgImageGenerator;
    ///
    /// let generator = OgImageGenerator::new(PathBuf::from("/usr/local/bin/typst"));
    /// ```
    pub fn new(typst_binary_path: PathBuf) -> Self {
        Self { typst_binary_path }
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
    pub fn from_environment() -> Result<Self, OgImageError> {
        if let Some(path) = var("TYPST_PATH").map_err(OgImageError::EnvVarError)? {
            Ok(Self::new(PathBuf::from(path)))
        } else {
            Ok(Self::default())
        }
    }

    /// Processes avatars by downloading URLs and copying assets to the assets directory.
    ///
    /// This method handles both asset-based avatars (which are copied from the bundled assets)
    /// and URL-based avatars (which are downloaded from the internet).
    /// Returns a mapping from avatar source to the local filename.
    async fn process_avatars<'a>(
        &self,
        data: &'a OgImageData<'_>,
        assets_dir: &std::path::Path,
    ) -> Result<HashMap<&'a str, String>, OgImageError> {
        let mut avatar_map = HashMap::new();

        let client = reqwest::Client::new();
        for (index, author) in data.authors.iter().enumerate() {
            if let Some(avatar) = &author.avatar {
                let filename = format!("avatar_{index}.png");
                let avatar_path = assets_dir.join(&filename);

                // Get the bytes either from the included asset or download from URL
                let bytes = if *avatar == "test-avatar" {
                    // Copy directly from included bytes
                    Bytes::from_static(include_bytes!("../assets/test-avatar.png"))
                } else {
                    // Download the avatar from the URL
                    let response = client.get(*avatar).send().await.map_err(|err| {
                        OgImageError::AvatarDownloadError {
                            url: (*avatar).to_string(),
                            source: err,
                        }
                    })?;

                    let bytes = response.bytes().await;
                    bytes.map_err(|err| OgImageError::AvatarDownloadError {
                        url: (*avatar).to_string(),
                        source: err,
                    })?
                };

                // Write the bytes to the avatar file
                fs::write(&avatar_path, bytes).await.map_err(|err| {
                    OgImageError::AvatarWriteError {
                        path: avatar_path.clone(),
                        source: err,
                    }
                })?;

                // Store the mapping from the avatar source to the numbered filename
                avatar_map.insert(*avatar, filename);
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
    ///     version: "v1.0.0",
    ///     description: "A sample crate",
    ///     license: "MIT",
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
    pub async fn generate(&self, data: OgImageData<'_>) -> Result<NamedTempFile, OgImageError> {
        // Create a temporary folder
        let temp_dir = tempfile::tempdir().map_err(OgImageError::TempDirError)?;

        // Create assets directory and copy logo and icons
        let assets_dir = temp_dir.path().join("assets");
        fs::create_dir(&assets_dir).await?;
        let cargo_logo = include_bytes!("../assets/cargo.png");
        fs::write(assets_dir.join("cargo.png"), cargo_logo).await?;
        let rust_logo_svg = include_bytes!("../assets/rust-logo.svg");
        fs::write(assets_dir.join("rust-logo.svg"), rust_logo_svg).await?;

        // Copy SVG icons
        let code_branch_svg = include_bytes!("../assets/code-branch.svg");
        fs::write(assets_dir.join("code-branch.svg"), code_branch_svg).await?;
        let code_svg = include_bytes!("../assets/code.svg");
        fs::write(assets_dir.join("code.svg"), code_svg).await?;
        let scale_balanced_svg = include_bytes!("../assets/scale-balanced.svg");
        fs::write(assets_dir.join("scale-balanced.svg"), scale_balanced_svg).await?;
        let tag_svg = include_bytes!("../assets/tag.svg");
        fs::write(assets_dir.join("tag.svg"), tag_svg).await?;
        let weight_hanging_svg = include_bytes!("../assets/weight-hanging.svg");
        fs::write(assets_dir.join("weight-hanging.svg"), weight_hanging_svg).await?;

        // Process avatars - download URLs and copy assets
        let avatar_map = self.process_avatars(&data, &assets_dir).await?;

        // Copy the static Typst template file
        let template_content = include_str!("../templates/og-image.typ");
        let typ_file_path = temp_dir.path().join("og-image.typ");
        fs::write(&typ_file_path, template_content).await?;

        // Create a named temp file for the output PNG
        let output_file = NamedTempFile::new().map_err(OgImageError::TempFileError)?;

        // Serialize data and avatar_map to JSON
        let json_data = serde_json::to_string(&data);
        let json_data = json_data.map_err(OgImageError::JsonSerializationError)?;

        let json_avatar_map = serde_json::to_string(&avatar_map);
        let json_avatar_map = json_avatar_map.map_err(OgImageError::JsonSerializationError)?;

        // Run typst compile command with input data
        let output = Command::new(&self.typst_binary_path)
            .arg("compile")
            .arg("--format")
            .arg("png")
            .arg("--input")
            .arg(format!("data={json_data}"))
            .arg("--input")
            .arg(format!("avatar_map={json_avatar_map}"))
            .arg(&typ_file_path)
            .arg(output_file.path())
            .output()
            .await
            .map_err(OgImageError::TypstNotFound)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            return Err(OgImageError::TypstCompilationError {
                stderr,
                stdout,
                exit_code: output.status.code(),
            });
        }

        Ok(output_file)
    }
}

impl Default for OgImageGenerator {
    /// Creates a default `OgImageGenerator` that assumes the Typst binary is available
    /// as "typst" in the system PATH.
    fn default() -> Self {
        Self {
            typst_binary_path: PathBuf::from("typst"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn author(name: &str) -> OgImageAuthorData<'_> {
        OgImageAuthorData::new(name, None)
    }

    const fn author_with_avatar(name: &str) -> OgImageAuthorData<'_> {
        OgImageAuthorData::new(name, Some("test-avatar"))
    }

    fn create_minimal_test_data() -> OgImageData<'static> {
        static AUTHORS: &[OgImageAuthorData<'_>] = &[author("author")];

        OgImageData {
            name: "minimal-crate",
            version: "v1.0.0",
            description: "A minimal crate",
            license: "MIT",
            tags: &[],
            authors: AUTHORS,
            lines_of_code: None,
            crate_size: 10000,
            releases: 1,
        }
    }

    fn create_escaping_test_data() -> OgImageData<'static> {
        static AUTHORS: &[OgImageAuthorData<'_>] = &[
            author_with_avatar("author \"with quotes\""),
            author("author\\with\\backslashes"),
            author("author#with#hashes"),
        ];

        OgImageData {
            name: "crate-with-\"quotes\"",
            version: "v1.0.0-\"beta\"",
            description: "A crate with \"quotes\", \\ backslashes, and other special chars: #[]{}()",
            license: "MIT OR \"Apache-2.0\"",
            tags: &[
                "tag-with-\"quotes\"",
                "tag\\with\\backslashes",
                "tag#with#symbols",
            ],
            authors: AUTHORS,
            lines_of_code: Some(42),
            crate_size: 256256,
            releases: 5,
        }
    }

    fn create_overflow_test_data() -> OgImageData<'static> {
        static AUTHORS: &[OgImageAuthorData<'_>] = &[
            author_with_avatar("alice-wonderland"),
            author("bob-the-builder"),
            author_with_avatar("charlie-brown"),
            author("diana-prince"),
            author_with_avatar("edward-scissorhands"),
            author("fiona-apple"),
            author("george-washington"),
            author_with_avatar("helen-keller"),
            author("isaac-newton"),
            author("jane-doe"),
        ];

        OgImageData {
            name: "super-long-crate-name-for-testing-overflow-behavior",
            version: "v2.1.0-beta.1+build.12345",
            description: "This is an extremely long description that tests how the layout handles descriptions that might wrap to multiple lines or overflow the available space in the OpenGraph image template design. This is an extremely long description that tests how the layout handles descriptions that might wrap to multiple lines or overflow the available space in the OpenGraph image template design.",
            license: "MIT/Apache-2.0/ISC/BSD-3-Clause",
            tags: &[
                "web-framework",
                "async-runtime",
                "database-orm",
                "serialization",
                "networking",
            ],
            authors: AUTHORS,
            lines_of_code: Some(147000),
            crate_size: 2847123,
            releases: 1432,
        }
    }

    fn create_simple_test_data() -> OgImageData<'static> {
        static AUTHORS: &[OgImageAuthorData<'_>] = &[author("test-user")];

        OgImageData {
            name: "test-crate",
            version: "v1.0.0",
            description: "A test crate for OpenGraph image generation",
            license: "MIT/Apache-2.0",
            tags: &["testing", "og-image"],
            authors: AUTHORS,
            lines_of_code: Some(1000),
            crate_size: 42012,
            releases: 1,
        }
    }

    fn skip_if_typst_unavailable() -> bool {
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

        let generator = OgImageGenerator::default();

        let temp_file = generator
            .generate(data)
            .await
            .expect("Failed to generate image");

        Some(std::fs::read(temp_file.path()).expect("Failed to read generated image"))
    }

    #[tokio::test]
    async fn test_generate_og_image_snapshot() {
        let data = create_simple_test_data();

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_overflow_snapshot() {
        let data = create_overflow_test_data();

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image_overflow.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_minimal_snapshot() {
        let data = create_minimal_test_data();

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image_minimal.png", image_data);
        }
    }

    #[tokio::test]
    async fn test_generate_og_image_escaping_snapshot() {
        let data = create_escaping_test_data();

        if let Some(image_data) = generate_image(data).await {
            insta::assert_binary_snapshot!("generated_og_image_escaping.png", image_data);
        }
    }
}

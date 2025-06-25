//! OpenGraph image generation for crates.io

use anyhow::anyhow;
use crates_io_env_vars::var;
use minijinja::{Environment, context};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::LazyLock;
use tempfile::NamedTempFile;
use tokio::process::Command;

static TEMPLATE_ENV: LazyLock<Environment<'_>> = LazyLock::new(|| {
    let mut env = Environment::new();
    let template_str = include_str!("../templates/og-image.typ.j2");
    env.add_template("og-image.typ", template_str).unwrap();
    env
});

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
    pub lines_of_code: Option<u32>,
    /// Package size in bytes
    pub crate_size: u32,
    /// Total number of releases
    pub releases: u32,
}

/// Author information for OpenGraph image generation
#[derive(Debug, Clone, Serialize)]
pub struct OgImageAuthorData<'a> {
    /// Author username/name
    pub name: &'a str,
    /// Optional path to avatar image file
    pub avatar: Option<&'a str>,
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
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn from_environment() -> anyhow::Result<Self> {
        if let Some(path) = var("TYPST_PATH")? {
            Ok(Self::new(PathBuf::from(path)))
        } else {
            Ok(Self::default())
        }
    }

    /// Generates the Typst template content from the provided data.
    ///
    /// This private method renders the Jinja2 template with the provided data
    /// and returns the resulting Typst markup as a string.
    fn generate_template(&self, data: OgImageData<'_>) -> anyhow::Result<String> {
        let template = TEMPLATE_ENV.get_template("og-image.typ")?;
        let rendered = template.render(context! { data })?;
        Ok(rendered)
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
    /// use crates_io_og_image::{OgImageGenerator, OgImageData, OgImageAuthorData};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
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
    pub async fn generate(&self, data: OgImageData<'_>) -> anyhow::Result<NamedTempFile> {
        // Create a temporary folder
        let temp_dir = tempfile::tempdir()?;

        // Create assets directory and copy logo
        let assets_dir = temp_dir.path().join("assets");
        std::fs::create_dir(&assets_dir)?;
        let cargo_logo = include_bytes!("../assets/cargo.png");
        std::fs::write(assets_dir.join("cargo.png"), cargo_logo)?;

        // Create og-image.typ file using minijinja template
        let rendered = self.generate_template(data)?;
        let typ_file_path = temp_dir.path().join("og-image.typ");
        std::fs::write(&typ_file_path, rendered)?;

        // Create a named temp file for the output PNG
        let output_file = NamedTempFile::new()?;

        // Run typst compile command
        let output = Command::new(&self.typst_binary_path)
            .arg("compile")
            .arg("--format")
            .arg("png")
            .arg(&typ_file_path)
            .arg(output_file.path())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("typst compile failed: {stderr}"));
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

    #[test]
    fn test_generate_template_snapshot() {
        let generator = OgImageGenerator::default();
        let data = OgImageData {
            name: "example-crate",
            version: "v2.1.0",
            description: "A comprehensive example crate showcasing various OpenGraph features",
            license: "MIT OR Apache-2.0",
            tags: &["web", "api", "async", "json", "http"],
            authors: &[
                OgImageAuthorData {
                    name: "alice",
                    avatar: Some("avatar1.png"),
                },
                OgImageAuthorData {
                    name: "bob",
                    avatar: None,
                },
            ],
            lines_of_code: Some(5500),
            crate_size: 128,
            releases: 15,
        };

        let template_content = generator
            .generate_template(data)
            .expect("Failed to generate template");

        // Use insta to create a snapshot of the generated Typst template
        insta::assert_snapshot!("generated_template.typ", template_content);
    }

    #[test]
    fn test_generate_template_minimal_snapshot() {
        let generator = OgImageGenerator::default();
        let data = OgImageData {
            name: "minimal-crate",
            version: "v1.0.0",
            description: "A minimal crate",
            license: "MIT",
            tags: &[],
            authors: &[OgImageAuthorData {
                name: "author",
                avatar: None,
            }],
            lines_of_code: None,
            crate_size: 10,
            releases: 1,
        };

        let template_content = generator
            .generate_template(data)
            .expect("Failed to generate template");

        // Use insta to create a snapshot of the generated Typst template
        insta::assert_snapshot!("generated_template_minimal.typ", template_content);
    }

    #[tokio::test]
    async fn test_generate_og_image_snapshot() {
        // Skip test if typst is not available
        if std::process::Command::new("typst")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping test: typst binary not found in PATH");
            return;
        }

        let generator = OgImageGenerator::default();
        let data = OgImageData {
            name: "test-crate",
            version: "v1.0.0",
            description: "A test crate for OpenGraph image generation",
            license: "MIT/Apache-2.0",
            tags: &["testing", "og-image"],
            authors: &[OgImageAuthorData {
                name: "test-user",
                avatar: None,
            }],
            lines_of_code: Some(1000),
            crate_size: 42,
            releases: 1,
        };

        let temp_file = generator
            .generate(data)
            .await
            .expect("Failed to generate image");
        let image_data = std::fs::read(temp_file.path()).expect("Failed to read generated image");

        // Use insta to create a binary snapshot of the generated PNG
        insta::assert_binary_snapshot!("generated_og_image.png", image_data);
    }
}

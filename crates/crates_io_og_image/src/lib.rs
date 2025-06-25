//! OpenGraph image generation for crates.io

use anyhow::anyhow;
use crates_io_env_vars::var;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::process::Command;

/// Data structure containing information needed to generate an OpenGraph image
/// for a crates.io crate.
pub struct OgImageData {
    // Placeholder for now
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

    /// Generates an OpenGraph image using the provided data.
    ///
    /// This method creates a temporary directory with all the necessary files
    /// to create the OpenGraph image, compiles it to PNG using the Typst
    /// binary, and returns the resulting image as a `NamedTempFile`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crates_io_og_image::{OgImageGenerator, OgImageData};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let generator = OgImageGenerator::default();
    /// let data = OgImageData {};
    /// let image_file = generator.generate(data).await?;
    /// println!("Generated image at: {:?}", image_file.path());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, _data: OgImageData) -> anyhow::Result<NamedTempFile> {
        // Create a temporary folder
        let temp_dir = tempfile::tempdir()?;

        // Create a basic og-image.typ file in the temporary folder
        let typ_file_path = temp_dir.path().join("og-image.typ");
        std::fs::write(&typ_file_path, "Hello World")?;

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

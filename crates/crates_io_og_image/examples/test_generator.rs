use crates_io_og_image::{OgImageAuthorData, OgImageData, OgImageGenerator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OgImageGenerator...");

    let generator = OgImageGenerator::from_environment()?;
    println!("Created generator from environment");

    // Test generating an image
    let data = OgImageData {
        name: "example-crate",
        version: "v1.2.3",
        description: "An example crate for testing OpenGraph image generation",
        license: "MIT/Apache-2.0",
        tags: &["example", "testing", "og-image"],
        authors: &[
            OgImageAuthorData::new("example-user", None),
            OgImageAuthorData::with_url(
                "Turbo87",
                "https://avatars.githubusercontent.com/u/141300",
            ),
        ],
        lines_of_code: Some(2000),
        crate_size: 75,
        releases: 5,
    };
    match generator.generate(data).await {
        Ok(temp_file) => {
            let output_path = "test_og_image.png";
            std::fs::copy(temp_file.path(), output_path)?;
            println!("Successfully generated image at: {output_path}");
            println!(
                "Image file size: {} bytes",
                std::fs::metadata(output_path)?.len()
            );
        }
        Err(error) => {
            println!("Failed to generate image: {error}");
            println!("Make sure typst is installed and available in PATH");
        }
    }

    Ok(())
}

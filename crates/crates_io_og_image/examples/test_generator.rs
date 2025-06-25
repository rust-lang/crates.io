use crates_io_og_image::{OgImageData, OgImageGenerator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OgImageGenerator...");

    let generator = OgImageGenerator::from_environment()?;
    println!("Created generator from environment");

    // Test generating an image
    let data = OgImageData {};
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

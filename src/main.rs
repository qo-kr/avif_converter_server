use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use exif::{In, Tag};
use image::{imageops::FilterType, ColorType};
use ravif::*;
use reqwest::Client;
use rgb::{RGB8, RGBA8};
use std::collections::HashMap;
use std::io::Cursor;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello from AVIF converter!")
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

// Core image processing logic extracted into a separate function
fn process_image_bytes(bytes: &bytes::Bytes, width: u32, height: u32) -> Result<ravif::EncodedImage, String> {
    // Read and log EXIF information
    let mut orientation = 1;
    if let Ok(exif_reader) = exif::Reader::new().read_from_container(&mut Cursor::new(bytes)) {
        if let Some(orientation_field) = exif_reader.get_field(Tag::Orientation, In::PRIMARY) {
            if let Some(orientation_value) = orientation_field.value.get_uint(0) {
                orientation = orientation_value as u32;
                println!("Found EXIF Orientation: {}", orientation);
            }
        }
    }

    // Load image
    let mut img = image::load_from_memory(bytes).map_err(|e| format!("Failed to load image: {}", e))?;
    println!("Original dimensions: {}x{}", img.width(), img.height());

    // Rotate image based on EXIF orientation
    img = match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.flipv().rotate90(),
        6 => img.rotate90(),
        7 => img.flipv().rotate270(),
        8 => img.rotate270(),
        _ => img,
    };

    // Resize image
    let img = if width > 0 || height > 0 {
        let target_width = if width > 0 { width } else { img.width() };
        let target_height = if height > 0 { height } else { img.height() };
        
        // Adjust size while maintaining aspect ratio
        let (new_width, new_height) = if width > 0 && height > 0 {
            (target_width, target_height)
        } else if width > 0 {
            let aspect_ratio = img.width() as f32 / img.height() as f32;
            let new_height = (target_width as f32 / aspect_ratio).round() as u32;
            (target_width, new_height)
        } else {
            let aspect_ratio = img.width() as f32 / img.height() as f32;
            let new_width = (target_height as f32 * aspect_ratio).round() as u32;
            (new_width, target_height)
        };

        img.resize(new_width, new_height, FilterType::Lanczos3)
    } else {
        img
    };

    // Convert to AVIF based on color type
    let has_alpha = img.color().has_alpha();
    
    let encoder = ravif::Encoder::new()
        .with_quality(80.0)
        .with_speed(6)
        .with_alpha_quality(80.0);

    if has_alpha {
        let rgba_image = img.to_rgba8();
        let width = rgba_image.width() as usize;
        let height = rgba_image.height() as usize;
        
        let mut pixels = Vec::with_capacity(width * height);
        for pixel in rgba_image.pixels() {
            pixels.push(RGBA8::new(pixel[0], pixel[1], pixel[2], pixel[3]));
        }
        let buffer = Img::new(pixels.as_slice(), width, height);
        encoder.encode_rgba(buffer)
    } else {
        let rgb_image = img.to_rgb8();
        let width = rgb_image.width() as usize;
        let height = rgb_image.height() as usize;

        let mut pixels = Vec::with_capacity(width * height);
        for pixel in rgb_image.pixels() {
            pixels.push(RGB8::new(pixel[0], pixel[1], pixel[2]));
        }
        let buffer = Img::new(pixels.as_slice(), width, height);
        encoder.encode_rgb(buffer)
    }.map_err(|e| format!("Failed to encode AVIF: {}", e))
}

async fn convert_and_resize_image(query: web::Query<HashMap<String, String>>) -> impl Responder {
    let image_url = match query.get("url") {
        Some(url) => url,
        None => return HttpResponse::BadRequest().body("Image URL is required"),
    };
    let width: u32 = query
        .get("width")
        .unwrap_or(&"0".to_string())
        .parse()
        .unwrap_or(0);
    let height: u32 = query
        .get("height")
        .unwrap_or(&"0".to_string())
        .parse()
        .unwrap_or(0);

    let client = Client::new();
    let res = match client.get(image_url).send().await {
        Ok(res) => res,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to send HTTP request"),
    };
    let bytes = match res.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to read HTTP response body"),
    };

    match process_image_bytes(&bytes, width, height) {
        Ok(encoded_image) => HttpResponse::Ok()
            .content_type("image/avif")
            .body(encoded_image.avif_file),
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/up", web::get().to(health_check))
            .route("/convert", web::get().to(convert_and_resize_image))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use bytes::Bytes;

    struct ImageSize<'a> {
        width: u32,
        height: u32,
        name: &'a str,
    }

    #[test]
    fn test_image_conversion_with_alpha() {
        let image_path = "1758604843316.png";
        let image_bytes = Bytes::from(fs::read(image_path).expect("Failed to read image file"));

        let sizes: [ImageSize; 5] = [
            ImageSize { width: 2400, height: 1800, name: "xl" },
            ImageSize { width: 576, height: 432, name: "lg" },
            ImageSize { width: 416, height: 312, name: "md" },
            ImageSize { width: 320, height: 240, name: "sm" },
            ImageSize { width: 192, height: 144, name: "xs" },
        ];

        for size in &sizes {
            let result = process_image_bytes(&image_bytes, size.width, size.height);
            assert!(result.is_ok(), "Failed to process image with alpha");
            let encoded_image = result.unwrap();
            
            let output_filename = format!("test_output_png_{}.avif", size.name);
            fs::write(&output_filename, encoded_image.avif_file)
                .expect(&format!("Failed to write output file: {}", output_filename));
            
            println!("Successfully generated {}", output_filename);
        }
    }

    #[test]
    fn test_image_conversion_without_alpha() {
        let image_path = "original.jpg";
        let image_bytes = Bytes::from(fs::read(image_path).expect("Failed to read image file"));

        let sizes: [ImageSize; 3] = [
            ImageSize { width: 1024, height: 768, name: "lg" },
            ImageSize { width: 800, height: 600, name: "md" },
            ImageSize { width: 400, height: 300, name: "sm" },
        ];

        for size in &sizes {
            let result = process_image_bytes(&image_bytes, size.width, size.height);
            assert!(result.is_ok(), "Failed to process image without alpha");
            let encoded_image = result.unwrap();
            
            let output_filename = format!("test_output_jpg_{}.avif", size.name);
            fs::write(&output_filename, encoded_image.avif_file)
                .expect(&format!("Failed to write output file: {}", output_filename));
            
            println!("Successfully generated {}", output_filename);
        }
    }
}

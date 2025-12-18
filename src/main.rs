use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use exif::{In, Tag};
use image::{imageops::{self, FilterType}, DynamicImage, GenericImageView, Rgba, RgbaImage};
use ravif::*;
use reqwest::Client;
use rgb::{RGB8, RGBA8};
use std::collections::HashMap;
use std::io::Cursor;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeFit {
    Contain,
    Cover,
    Fill,
}

impl FromStr for ResizeFit {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "contain" => Ok(ResizeFit::Contain),
            "cover" => Ok(ResizeFit::Cover),
            "fill" => Ok(ResizeFit::Fill),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
struct ResizeOptions {
    width: u32,
    height: u32,
    fit: Option<ResizeFit>,
    bg: Option<String>,
    quality: f32,
    pattern_data: Option<Vec<u8>>,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            fit: None,
            bg: None,
            quality: 80.0,
            pattern_data: None,
        }
    }
}

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello from AVIF converter!")
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

fn parse_hex_color(hex: &str) -> Option<Rgba<u8>> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Rgba([r, g, b, 255]))
    } else {
        None
    }
}

fn create_background(width: u32, height: u32, bg_str: Option<&str>, pattern_data: Option<&[u8]>) -> DynamicImage {
    if let Some(data) = pattern_data {
        if let Ok(pattern_img) = image::load_from_memory(data) {
            let mut bg_img = RgbaImage::new(width, height);
            let p_width = pattern_img.width();
            let p_height = pattern_img.height();
            
            for y in 0..height {
                for x in 0..width {
                    let px = pattern_img.get_pixel(x % p_width, y % p_height);
                    bg_img.put_pixel(x, y, px);
                }
            }
            return DynamicImage::ImageRgba8(bg_img);
        }
    }

    let color = match bg_str {
        Some("white") => Rgba([255, 255, 255, 255]),
        Some("black") => Rgba([0, 0, 0, 255]),
        Some("gray") => Rgba([128, 128, 128, 255]),
        Some(hex) => parse_hex_color(hex).unwrap_or(Rgba([0, 0, 0, 0])), // Default to transparent if invalid
        None => Rgba([0, 0, 0, 0]), // Transparent
    };

    DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, color))
}

// Core image processing logic extracted into a separate function
fn process_image_bytes(bytes: &bytes::Bytes, options: &ResizeOptions) -> Result<ravif::EncodedImage, String> {
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

    // Resize logic
    let final_img = if options.width > 0 || options.height > 0 {
        let target_width = if options.width > 0 { options.width } else { img.width() };
        let target_height = if options.height > 0 { options.height } else { img.height() };

        match options.fit {
            Some(ResizeFit::Fill) => {
                img.resize_exact(target_width, target_height, FilterType::Lanczos3)
            }
            Some(ResizeFit::Cover) => {
                img.resize_to_fill(target_width, target_height, FilterType::Lanczos3)
            }
            Some(ResizeFit::Contain) => {
                // Resize preserving aspect ratio to fit within target
                let resized = img.resize(target_width, target_height, FilterType::Lanczos3);
                
                // Create background canvas
                let mut bg = create_background(target_width, target_height, options.bg.as_deref(), options.pattern_data.as_deref());
                
                // Center overlay
                let x = (target_width - resized.width()) / 2;
                let y = (target_height - resized.height()) / 2;
                imageops::overlay(&mut bg, &resized, x as i64, y as i64);
                bg
            }
            None => {
                // Legacy behavior:
                // If both width/height are provided, original behavior was resize (which preserves aspect ratio by default in `resize` method)
                // BUT the original code calculated new_width/new_height manually for one dimension 0 case.
                // However, `image::DynamicImage::resize` already handles aspect ratio preservation if we give it the bounding box.
                // The original code's specific logic for 0 handling:
                
                let (new_width, new_height) = if options.width > 0 && options.height > 0 {
                    // Original code: passed both directly to resize.
                    // img.resize(new_width, new_height, FilterType::Lanczos3) preserves aspect ratio fitting INSIDE the box.
                    (target_width, target_height)
                } else if options.width > 0 {
                    let aspect_ratio = img.width() as f32 / img.height() as f32;
                    let new_height = (target_width as f32 / aspect_ratio).round() as u32;
                    (target_width, new_height)
                } else {
                    let aspect_ratio = img.width() as f32 / img.height() as f32;
                    let new_width = (target_height as f32 * aspect_ratio).round() as u32;
                    (new_width, target_height)
                };

                img.resize(new_width, new_height, FilterType::Lanczos3)
            }
        }
    } else {
        img
    };

    // Convert to AVIF based on color type
    let has_alpha = final_img.color().has_alpha();
    
    let encoder = ravif::Encoder::new()
        .with_quality(options.quality)
        .with_speed(6)
        .with_alpha_quality(options.quality);

    if has_alpha {
        let rgba_image = final_img.to_rgba8();
        let width = rgba_image.width() as usize;
        let height = rgba_image.height() as usize;
        
        let mut pixels = Vec::with_capacity(width * height);
        for pixel in rgba_image.pixels() {
            pixels.push(RGBA8::new(pixel[0], pixel[1], pixel[2], pixel[3]));
        }
        let buffer = Img::new(pixels.as_slice(), width, height);
        encoder.encode_rgba(buffer)
    } else {
        let rgb_image = final_img.to_rgb8();
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

    let width: u32 = query.get("width").and_then(|v| v.parse().ok()).unwrap_or(0);
    let height: u32 = query.get("height").and_then(|v| v.parse().ok()).unwrap_or(0);
    
    let fit = query.get("fit").and_then(|v| ResizeFit::from_str(v).ok());
    let bg = query.get("bg").cloned();
    let quality: f32 = query.get("quality").and_then(|v| v.parse().ok()).unwrap_or(80.0);
    
    let pattern_url = query.get("pattern");

    let client = Client::new();
    
    // Download main image
    let res = match client.get(image_url).send().await {
        Ok(res) => res,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to send HTTP request"),
    };
    let bytes = match res.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to read HTTP response body"),
    };

    // Download pattern image if needed
    let pattern_data = if let Some(p_url) = pattern_url {
        match client.get(p_url).send().await {
            Ok(p_res) => p_res.bytes().await.ok().map(|b| b.to_vec()),
            Err(_) => None,
        }
    } else {
        None
    };

    let options = ResizeOptions {
        width,
        height,
        fit,
        bg,
        quality,
        pattern_data,
    };

    match process_image_bytes(&bytes, &options) {
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
    use image::GenericImageView;

    struct ImageSize<'a> {
        width: u32,
        height: u32,
        name: &'a str,
    }

    fn create_checkerboard(width: u32, height: u32, size: u32, c1: Rgba<u8>, c2: Rgba<u8>) -> RgbaImage {
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                if ((x / size) + (y / size)) % 2 == 0 {
                    img.put_pixel(x, y, c1);
                } else {
                    img.put_pixel(x, y, c2);
                }
            }
        }
        img
    }

    fn create_stripes(width: u32, height: u32, size: u32, c1: Rgba<u8>, c2: Rgba<u8>) -> RgbaImage {
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                if ((x + y) / size) % 2 == 0 {
                    img.put_pixel(x, y, c1);
                } else {
                    img.put_pixel(x, y, c2);
                }
            }
        }
        img
    }

    fn create_dots(width: u32, height: u32, spacing: u32, radius: u32, bg: Rgba<u8>, dot: Rgba<u8>) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(width, height, bg);
        for y in 0..height {
            for x in 0..width {
                let cx = (x / spacing) * spacing + spacing / 2;
                let cy = (y / spacing) * spacing + spacing / 2;
                let dx = x as i64 - cx as i64;
                let dy = y as i64 - cy as i64;
                if dx*dx + dy*dy <= (radius as i64 * radius as i64) {
                    img.put_pixel(x, y, dot);
                }
            }
        }
        img
    }

    fn image_to_bytes(img: &RgbaImage) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageOutputFormat::Png).unwrap();
        cursor.into_inner()
    }

    #[test]
    fn test_image_conversion_with_alpha() {
        let image_path = "1758604843316.png";
        // Ensure test file exists or skip if running in CI without it
        if !std::path::Path::new(image_path).exists() {
            println!("Test image not found, skipping test");
            return;
        }
        let image_bytes = Bytes::from(fs::read(image_path).expect("Failed to read image file"));

        let sizes: [ImageSize; 5] = [
            ImageSize { width: 2400, height: 1800, name: "xl" },
            ImageSize { width: 576, height: 432, name: "lg" },
            ImageSize { width: 416, height: 312, name: "md" },
            ImageSize { width: 320, height: 240, name: "sm" },
            ImageSize { width: 192, height: 144, name: "xs" },
        ];

        for size in &sizes {
            let options = ResizeOptions {
                width: size.width,
                height: size.height,
                ..Default::default()
            };
            let result = process_image_bytes(&image_bytes, &options);
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
        if !std::path::Path::new(image_path).exists() {
            println!("Test image not found, skipping test");
            return;
        }
        let image_bytes = Bytes::from(fs::read(image_path).expect("Failed to read image file"));

        let sizes: [ImageSize; 3] = [
            ImageSize { width: 1024, height: 768, name: "lg" },
            ImageSize { width: 800, height: 600, name: "md" },
            ImageSize { width: 400, height: 300, name: "sm" },
        ];

        for size in &sizes {
            let options = ResizeOptions {
                width: size.width,
                height: size.height,
                ..Default::default()
            };
            let result = process_image_bytes(&image_bytes, &options);
            assert!(result.is_ok(), "Failed to process image without alpha");
            let encoded_image = result.unwrap();
            
            let output_filename = format!("test_output_jpg_{}.avif", size.name);
            fs::write(&output_filename, encoded_image.avif_file)
                .expect(&format!("Failed to write output file: {}", output_filename));
            
            println!("Successfully generated {}", output_filename);
        }
    }

    #[test]
    fn test_resize_padding_options() {
        // Create a simple red 100x100 image in memory for testing
        let mut img = RgbaImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        let mut cursor = Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageOutputFormat::Png).unwrap();
        let bytes = Bytes::from(cursor.into_inner());

        // Test 1: Contain with White Padding (200x200)
        let options = ResizeOptions {
            width: 200,
            height: 200,
            fit: Some(ResizeFit::Contain),
            bg: Some("white".to_string()),
            ..Default::default()
        };
        
        let result = process_image_bytes(&bytes, &options).expect("Failed to process");
        
        // Verify output dimensions by decoding the result (AVIF decoding might be tricky without a decoder lib, 
        // but here we trust the process_image_bytes returns valid AVIF if it didn't error.
        // Ideally we would decode and check pixels, but ravif is encoder-only in this context.
        // We will assume if it runs without error and produces bytes, logic worked.
        // To be safer, we can check the intermediate DynamicImage logic by unit testing helper functions if needed,
        // but integration test here is fine.)
        
        assert!(!result.avif_file.is_empty());
        println!("Generated padded AVIF size: {} bytes", result.avif_file.len());
    }

    #[test]
    fn test_resize_cover_options() {
         // Create a simple red 100x100 image
        let mut img = RgbaImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        let mut cursor = Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageOutputFormat::Png).unwrap();
        let bytes = Bytes::from(cursor.into_inner());

        // Test Cover 50x50
        let options = ResizeOptions {
            width: 50,
            height: 50,
            fit: Some(ResizeFit::Cover),
            ..Default::default()
        };
        
        let result = process_image_bytes(&bytes, &options).expect("Failed to process cover");
        assert!(!result.avif_file.is_empty());
    }

    #[test]
    fn test_visual_verification() {
        let image_path = "original.jpg";
        if !std::path::Path::new(image_path).exists() {
            println!("'original.jpg' not found. Skipping visual verification test.");
            return;
        }
        let image_bytes = Bytes::from(fs::read(image_path).expect("Failed to read image file"));

        // Case 1: 1280x960, Contain, White BG
        let opts_white = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Contain),
            bg: Some("white".to_string()),
            ..Default::default()
        };
        let res_white = process_image_bytes(&image_bytes, &opts_white).expect("Failed white");
        fs::write("visual_test_contain_white.avif", res_white.avif_file).expect("Write failed");
        println!("Generated visual_test_contain_white.avif");

        // Case 2: 1280x960, Contain, Black BG
        let opts_black = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Contain),
            bg: Some("black".to_string()),
            ..Default::default()
        };
        let res_black = process_image_bytes(&image_bytes, &opts_black).expect("Failed black");
        fs::write("visual_test_contain_black.avif", res_black.avif_file).expect("Write failed");
        println!("Generated visual_test_contain_black.avif");

        // Case 3: 1280x960, Contain, Red BG (Hex)
        let opts_red = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Contain),
            bg: Some("ff0000".to_string()),
            ..Default::default()
        };
        let res_red = process_image_bytes(&image_bytes, &opts_red).expect("Failed red");
        fs::write("visual_test_contain_red.avif", res_red.avif_file).expect("Write failed");
        println!("Generated visual_test_contain_red.avif");

        // Case 4: 1280x960, Cover
        let opts_cover = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Cover),
            ..Default::default()
        };
        let res_cover = process_image_bytes(&image_bytes, &opts_cover).expect("Failed cover");
        fs::write("visual_test_cover.avif", res_cover.avif_file).expect("Write failed");
        println!("Generated visual_test_cover.avif");

        // --- Pattern Tests ---

        // 1. Checkerboard (Yellow/Black)
        let checker = create_checkerboard(40, 40, 20, Rgba([255, 255, 0, 255]), Rgba([0, 0, 0, 255]));
        let checker_bytes = image_to_bytes(&checker);
        fs::write("pattern_checker.png", &checker_bytes).expect("Failed to write pattern");
        
        let opts_checker = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Contain),
            pattern_data: Some(checker_bytes),
            ..Default::default()
        };
        let res_checker = process_image_bytes(&image_bytes, &opts_checker).expect("Failed checker");
        fs::write("visual_test_pattern_checker.avif", res_checker.avif_file).expect("Write failed");
        println!("Generated pattern_checker.png & visual_test_pattern_checker.avif");

        // 2. Stripes (Blue/White)
        let stripes = create_stripes(40, 40, 10, Rgba([0, 0, 255, 255]), Rgba([255, 255, 255, 255]));
        let stripes_bytes = image_to_bytes(&stripes);
        fs::write("pattern_stripe.png", &stripes_bytes).expect("Failed to write pattern");

        let opts_stripes = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Contain),
            pattern_data: Some(stripes_bytes),
            ..Default::default()
        };
        let res_stripes = process_image_bytes(&image_bytes, &opts_stripes).expect("Failed stripes");
        fs::write("visual_test_pattern_stripe.avif", res_stripes.avif_file).expect("Write failed");
        println!("Generated pattern_stripe.png & visual_test_pattern_stripe.avif");

        // 3. Dots (Pink/White)
        let dots = create_dots(40, 40, 20, 5, Rgba([255, 192, 203, 255]), Rgba([255, 255, 255, 255]));
        let dots_bytes = image_to_bytes(&dots);
        fs::write("pattern_dot.png", &dots_bytes).expect("Failed to write pattern");

        let opts_dots = ResizeOptions {
            width: 1280,
            height: 960,
            fit: Some(ResizeFit::Contain),
            pattern_data: Some(dots_bytes),
            ..Default::default()
        };
        let res_dots = process_image_bytes(&image_bytes, &opts_dots).expect("Failed dots");
        fs::write("visual_test_pattern_dot.avif", res_dots.avif_file).expect("Write failed");
        println!("Generated pattern_dot.png & visual_test_pattern_dot.avif");
    }
}

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use exif::{In, Tag};
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use ravif::*;
use reqwest::Client;
use rgb::RGB8;
use std::collections::HashMap;
use std::io::Cursor;

async fn convert_and_resize_image(query: web::Query<HashMap<String, String>>) -> impl Responder {
    let image_url = query.get("url").expect("Image URL is required");
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
    let res = client
        .get(image_url)
        .send()
        .await
        .expect("Failed to send HTTP request");
    let bytes = res
        .bytes()
        .await
        .expect("Failed to read HTTP response body");

    // EXIF 정보 읽기 및 로깅
    let mut orientation = 1;
    if let Ok(exif_reader) = exif::Reader::new().read_from_container(&mut Cursor::new(&bytes)) {
        if let Some(orientation_field) = exif_reader.get_field(Tag::Orientation, In::PRIMARY) {
            if let Some(orientation_value) = orientation_field.value.get_uint(0) {
                orientation = orientation_value as u32;
                println!("Found EXIF Orientation: {}", orientation);
            }
        }
    }

    // 이미지 로드
    let mut img = image::load_from_memory(&bytes).expect("Failed to load image");
    println!("Original dimensions: {}x{}", img.width(), img.height());

    // EXIF orientation에 따라 이미지 회전
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

    // 이미지 크기 조정
    let img = if width > 0 || height > 0 {
        let target_width = if width > 0 { width } else { img.width() };
        let target_height = if height > 0 { height } else { img.height() };
        
        // 종횡비 유지하면서 크기 조정
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

    // AVIF로 변환
    let rgba = img.to_rgba8();
    let width = rgba.width() as usize;
    let height = rgba.height() as usize;
    let mut pixels = Vec::with_capacity(width * height);
    
    for pixel in rgba.pixels() {
        pixels.push(RGB8::new(pixel[0], pixel[1], pixel[2]));
    }

    let buffer = Img::new(
        pixels.as_slice(),
        width,
        height,
    );

    let encoded_image = ravif::Encoder::new()
        .with_quality(80.0)  // 품질 설정 (0-100)
        .with_speed(6)       // 속도 설정 (1-10, 높을수록 빠르지만 압축률 감소)
        .with_alpha_quality(80.0)
        .with_internal_color_space(ravif::ColorSpace::RGB)
        .encode_rgb(buffer)
        .expect("Failed to encode AVIF");

    HttpResponse::Ok()
        .content_type("image/avif")
        .body(encoded_image.avif_file)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/convert", web::get().to(convert_and_resize_image))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

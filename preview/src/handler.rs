use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use log::{error, warn};
use std::io::Cursor;
use std::str;

use async_nats::jetstream::Message;
use database::DbManager;
use image::{
    imageops::FilterType::Triangle, DynamicImage, GenericImageView, ImageDecoder, ImageReader,
    RgbImage,
};
use s3::Bucket;

// FIX: change this to the http crate
const CONTENT_TYPE_HEADER: &str = "Content-Type";
const PREVIEW_ID_PREFIX: &str = "prev/";
const IOS_MEDIA_TYPES: [&str; 2] = ["image/heif", "image/heic"];

pub async fn handle_request(msg: Message, bucket: Box<Bucket>, db: DbManager) {
    let payload_bytes: &[u8] = &msg.payload;
    let source_image_id = match str::from_utf8(payload_bytes) {
        Ok(path) => path.to_owned(),
        Err(err) => {
            error!("Couldn't convert image path into utf8: {err:?}");
            return;
        }
    };

    let source_image_response = match bucket.get_object(source_image_id.clone()).await {
        Ok(oir) => oir,
        Err(err) => {
            error!("Get object failed: {err}");
            return;
        }
    };

    let source_image_bytes = source_image_response.as_slice();
    let content_type = match source_image_response.headers().get(CONTENT_TYPE_HEADER) {
        Some(ct) => ct.clone(),
        None => {
            warn!("No content type provided in {source_image_id} object.");
            String::new()
        }
    };

    // FIX: create and add the other ios types
    let source_image = if IOS_MEDIA_TYPES.contains(&content_type.as_str()) {
        let lib_heif = LibHeif::new();
        let heif_context = match HeifContext::read_from_bytes(source_image_bytes) {
            Ok(ctx) => ctx,
            Err(err) => {
                error!("Error reading heif image content: {err}");
                return;
            }
        };
        let handle = match heif_context.primary_image_handle() {
            Ok(handle) => handle,
            Err(err) => {
                error!("Error getting heif primary handle: {err}");
                return;
            }
        };

        let decoded_image = match lib_heif.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None) {
            Ok(decoded_image) => decoded_image,
            Err(err) => {
                error!("Couldn't decode heif image: {err}");
                return;
            }
        };

        let width = decoded_image.width();
        let height = decoded_image.height();
        let pixels = match decoded_image.planes().interleaved {
            Some(pixels) => pixels,
            None => {
                error!("Couldn't get pixels from decoded image.");
                return;
            }
        };
        let img_buffer = match RgbImage::from_raw(width, height, pixels.data.to_vec()) {
            Some(buffer) => buffer,
            None => {
                error!("Couldn't create image buffer from decoded image.");
                return;
            }
        };

        DynamicImage::ImageRgb8(img_buffer)
    } else {
        let source_reader =
            match ImageReader::new(Cursor::new(source_image_bytes)).with_guessed_format() {
                Ok(rd) => rd,
                Err(err) => {
                    error!("Couldn't convert image: {err}");
                    return;
                }
            };
        let mut decoder = match source_reader.into_decoder() {
            Ok(decoder) => decoder,
            Err(err) => {
                error!("Could not decode image: {err}");
                return;
            }
        };
        let orientation = match decoder.orientation() {
            Ok(orientation) => orientation,
            Err(err) => {
                error!("Could not get image orientation: {err}");
                return;
            }
        };
        let mut dynamic_image = match DynamicImage::from_decoder(decoder) {
            Ok(oi) => oi,
            Err(err) => {
                error!("Couldn't convert image: {err}");
                return;
            }
        };
        dynamic_image.apply_orientation(orientation);
        dynamic_image
    };

    // Create preview
    // FIX: the height value shouldn't be hardcoded
    let preview = create_preview(source_image, 200);

    // Convert image to bytes in jpg format
    let mut preview_bytes: Vec<u8> = Vec::new();
    let mut preview_content_type = "image/jpeg";
    let mut preview_format = image::ImageFormat::Jpeg;
    if preview.color().has_alpha() {
        let _ = preview.write_to(
            &mut Cursor::new(&mut preview_bytes),
            image::ImageFormat::Png,
        );
        preview_content_type = "image/png";
        preview_format = image::ImageFormat::Png;
    }
    let _ = preview.write_to(&mut Cursor::new(&mut preview_bytes), preview_format);

    preview_id.insert_str(0, PREVIEW_ID_PREFIX);
    let preview_response_data = match bucket
        .put_object_with_content_type(&preview_id, &preview_bytes, preview_content_type)
        .await
    {
        Ok(rp) => rp,
        Err(err) => {
            error!("Put preview object failed with: {err}");
            return;
        }
    };
    if preview_response_data.status_code() != 200 {
        error!(
            "Put preview object failed with status code: {}",
            preview_response_data.status_code()
        );
        return;
    }

    if let Err(err) = db.update_media_preview(source_image_id, preview_id).await {
        error!("{err}");
        return;
    }

    match msg.ack().await {
        Ok(()) => (),
        Err(err) => println!("Couldn't acknowledge message {err}"),
    }
}

// Function to handle EXIF orientation
//fn fix_orientation(image: DynamicImage) -> DynamicImage {
//    if let Some(exif_orientation) = get_exif_orientation(&image) {
//        match exif_orientation {
//            3 => image.rotate180(),
//            6 => image.rotate90(),
//            8 => image.rotate270(),
//            _ => image, // No rotation needed
//        }
//    } else {
//        image // No EXIF data found
//    }
//}

// Creates a preview of the image with the given height
// The preview will have the same aspect ratio as the original image
fn create_preview(orig: DynamicImage, preview_height: u32) -> DynamicImage {
    let (width, height) = orig.dimensions();
    let aspect_ratio = width as f32 / height as f32;
    let preview_width = (preview_height as f32 * aspect_ratio) as u32;
    orig.resize(preview_width, preview_height, Triangle)
}

use async_nats::jetstream::Message;
use database::DbManager;
use exif::{Exif, In, Reader, Tag, Value};
use log::error;
use s3::Bucket;
use std::io::Cursor;
use std::str;

pub async fn handle_request(msg: Message, bucket: Box<Bucket>, db: DbManager) {
    let payload_bytes: &[u8] = &msg.payload;
    let source_media_id = match str::from_utf8(payload_bytes) {
        Ok(path) => path.to_owned(),
        Err(err) => {
            error!("Couldn't convert media path into utf8: {err:?}");
            return;
        }
    };

    let source_media_response = match bucket.get_object(source_media_id.clone()).await {
        Ok(oir) => oir,
        Err(err) => {
            error!("Get object failed: {err}");
            return;
        }
    };

    let source_media_bytes = source_media_response.bytes();
    let mut bufreader = Cursor::new(source_media_bytes);
    let exifreader = Reader::new();

    match exifreader.read_from_container(&mut bufreader) {
        Ok(exifdata) => {
            let longitude = extract_longitude(&exifdata);
            let latitude = extract_latitude(&exifdata);
            let image_width = extract_image_width(&exifdata);
            let image_length = extract_image_length(&exifdata);
            let make = extract_make(&exifdata);
            let model = extract_model(&exifdata);
            let fnumber = extract_fnumber(&exifdata);
            let exposure_time = extract_exposure_time(&exifdata);
            let photographic_sensitivity = extract_photographic_sensitivity(&exifdata);
            let orientation = extract_orientation(&exifdata);

            let _ = db
                .insert_metadata(
                    source_media_id,
                    longitude,
                    latitude,
                    image_width,
                    image_length,
                    make,
                    model,
                    fnumber,
                    exposure_time,
                    photographic_sensitivity,
                    orientation,
                )
                .await;
            let _ = msg.ack_with(async_nats::jetstream::AckKind::Ack).await;
        }
        Err(_) => {
            let _ = msg.ack_with(async_nats::jetstream::AckKind::Term).await;
        }
    };
}

fn extract_longitude(exifdata: &Exif) -> Option<f64> {
    if let Some(field) = exifdata.get_field(Tag::GPSLongitude, In::PRIMARY) {
        if let Value::Rational(ref values) = field.value {
            if values.len() == 3 {
                let degrees = values[0].to_f64();
                let minutes = values[1].to_f64();
                let seconds = values[2].to_f64();
                let mut longitude = degrees + minutes / 60.0 + seconds / 3600.0;

                if let Some(ref_field) = exifdata.get_field(Tag::GPSLongitudeRef, In::PRIMARY) {
                    if let Value::Ascii(ref data) = ref_field.value {
                        if let Some(ref_value) = data.first() {
                            if ref_value == b"W" {
                                longitude = -longitude;
                            }
                        }
                    }
                }
                return Some(longitude);
            }
        }
    }
    None
}

fn extract_latitude(exifdata: &Exif) -> Option<f64> {
    if let Some(field) = exifdata.get_field(Tag::GPSLatitude, In::PRIMARY) {
        if let Value::Rational(ref values) = field.value {
            if values.len() == 3 {
                let degrees = values[0].to_f64();
                let minutes = values[1].to_f64();
                let seconds = values[2].to_f64();
                let mut latitude = degrees + minutes / 60.0 + seconds / 3600.0;

                if let Some(ref_field) = exifdata.get_field(Tag::GPSLatitudeRef, In::PRIMARY) {
                    if let Value::Ascii(ref data) = ref_field.value {
                        if let Some(ref_value) = data.first() {
                            if ref_value == b"S" {
                                latitude = -latitude;
                            }
                        }
                    }
                }
                return Some(latitude);
            }
        }
    }
    None
}

fn extract_image_width(exifdata: &Exif) -> Option<i32> {
    exifdata
        .get_field(Tag::ImageWidth, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .map(|value| value as i32)
}

fn extract_image_length(exifdata: &Exif) -> Option<i32> {
    exifdata
        .get_field(Tag::ImageLength, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .map(|value| value as i32)
}

fn extract_make(exifdata: &Exif) -> Option<String> {
    exifdata.get_field(Tag::Make, In::PRIMARY).map(|field| {
        field
            .value
            .display_as(field.tag)
            .to_string()
            .replace("\"", "")
    })
}

fn extract_model(exifdata: &Exif) -> Option<String> {
    exifdata.get_field(Tag::Model, In::PRIMARY).map(|field| {
        field
            .value
            .display_as(field.tag)
            .to_string()
            .replace("\"", "")
    })
}

fn extract_fnumber(exifdata: &Exif) -> Option<String> {
    exifdata
        .get_field(Tag::FNumber, In::PRIMARY)
        .map(|field| field.value.display_as(field.tag).to_string())
}

fn extract_exposure_time(exifdata: &Exif) -> Option<String> {
    exifdata
        .get_field(Tag::ExposureTime, In::PRIMARY)
        .map(|field| field.value.display_as(field.tag).to_string())
}

fn extract_photographic_sensitivity(exifdata: &Exif) -> Option<String> {
    exifdata
        .get_field(Tag::PhotographicSensitivity, In::PRIMARY)
        .map(|field| field.value.display_as(field.tag).to_string())
}

fn extract_orientation(exifdata: &Exif) -> Option<i32> {
    exifdata
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .map(|value| value as i32)
}

use log::{error, info, warn};
use std::error::Error;
use std::io::Cursor;
use std::str;

use async_nats::jetstream::Message;
use database::DbManager;
use futures_util::StreamExt;
use image::{imageops::FilterType::Triangle, DynamicImage, ImageReader};
use s3::{creds::Credentials, error::S3Error, Bucket, BucketConfiguration, Region};

// Flow to create a preview
// 1. Get event wiht uid of image from NATS
// 2. Get image from event, and fetch it form the object storage
// 3. Create image preview the image X
// 4. Save preview to object storage
// 5. Update entry in database

const CONTENT_TYPE_HEADER: &str = "ContentType";
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let nats_addr = "localhost:4222";
    let stream_name = String::from("previews");

    let db = match DbManager::new().await {
        Ok(database) => database,
        Err(err) => panic!("{}", err),
    };

    let bucket_name = "chronolens";
    let bucket = setup_bucket(bucket_name, "http://localhost:9000").await?;
    let client = match async_nats::connect(nats_addr).await {
        Ok(c) => c,
        Err(err) => {
            panic!("Couldn't connect nats client.{err}");
        }
    };

    let jetstream = async_nats::jetstream::new(client);
    let stream = jetstream
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: stream_name.clone(),
            max_messages: 10000,
            ..Default::default()
        })
        .await?;

    // FIX: make this a proper test
    let test_uuid = String::from("27aaaa1f-33be-4988-8334-59a1770fa0d7");
    let img = ImageReader::open("test.jpg")?.decode()?;

    // Convert image to bytes in jpg format
    let mut img_bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut img_bytes), image::ImageFormat::Jpeg)?;
    let response = bucket.put_object(test_uuid.clone(), &img_bytes).await?;
    if response.status_code() != 200 {
        panic!("put test object failed");
    }
    jetstream.publish(stream_name, test_uuid.into()).await?;

    // FIX: end of population -----------------------

    let consumer = stream
        .get_or_create_consumer(
            "preview_consumer",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("preview_consumer".to_string()),
                ..Default::default()
            },
        )
        .await?;

    let mut messages = consumer.messages().await?;
    while let Some(message) = messages.next().await {
        match message {
            Ok(msg) => {
                info!(
                    "Message received: {:?}",
                    String::from_utf8(msg.payload.to_vec())
                );

                // TODO: spawn a thread for each event
                let thread_bucket = bucket.clone();
                let thread_db = db.clone();
                tokio::spawn(async move { handle_message(msg, thread_bucket, thread_db).await });
            }
            Err(err) => {
                error!("Error receiving message: {err}");
            }
        }
    }
    return Ok(());
}

// TODO: add orignal images bucket
// redo code and break it into functions
// test it
async fn handle_message(msg: Message, bucket: Box<Bucket>, db: DbManager) {
    let payload_bytes: &[u8] = &msg.payload;
    let orig_image_id = match str::from_utf8(payload_bytes) {
        Ok(path) => path.to_owned(),
        Err(err) => {
            error!("Couldn't convert image path into utf8: {err:?}");
            return;
        }
    };

    let orig_image_response = match bucket.get_object(orig_image_id.clone()).await {
        Ok(oir) => oir,
        Err(err) => {
            error!("Get object failed: {err}");
            return;
        }
    };

    let content_type = match orig_image_response.headers().get(CONTENT_TYPE_HEADER) {
        Some(ct) => *ct,
        None => {
            warn!("No content type provided in {orig_image_id} object.");
            String::new()
        }
    };

    // FIX: create and add the other ios types
    let ios_types = ["image/heif", "image/heic"];
    if ios_types.contains(&content_type.as_str()) {
        // TODO: decode ios type media with libheif
    }

    let orig_reader =
        match ImageReader::new(Cursor::new(orig_image_response.as_slice())).with_guessed_format() {
            Ok(rd) => rd,
            Err(err) => {
                error!("Couldn't convert image: {err}");
                return;
            }
        };
    let orig_image = match orig_reader.decode() {
        Ok(oi) => oi,
        Err(err) => {
            error!("Couldn't convert image: {err}");
            return;
        }
    };

    // Create preview
    let preview = create_preview(orig_image, 640, 640);

    // Convert image to bytes in jpg format
    let mut preview_bytes: Vec<u8> = Vec::new();
    let _ = preview.write_to(
        &mut Cursor::new(&mut preview_bytes),
        image::ImageFormat::Jpeg,
    );

    // let preview_id = Uuid::new_v4().to_string();
    let mut preview_id = orig_image_id.clone();
    let preview_id_prefix = "prv_";
    preview_id.insert_str(0, preview_id_prefix);
    let preview_response_data = match bucket.put_object(preview_id, &preview_bytes).await {
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

    // TODO: db update

    match msg.ack().await {
        Ok(()) => (),
        Err(err) => println!("Couldn't acknowledge message {err}"),
    }
}

async fn setup_bucket(bucket_name: &str, endpoint: &str) -> Result<Box<Bucket>, S3Error> {
    // connect to s3 storage
    let region = Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: endpoint.to_string(),
    };
    // INFO: this credentials are fetched from the default location of the aws
    // credentials (~/.aws/credentials)
    let credentials = Credentials::default().expect("Credentials died");

    let mut bucket =
        Bucket::new(bucket_name, region.clone(), credentials.clone())?.with_path_style();

    if !bucket.exists().await? {
        bucket = Bucket::create_with_path_style(
            bucket_name,
            region,
            credentials,
            BucketConfiguration::default(),
        )
        .await?
        .bucket;
    }
    Ok(bucket)
}

fn create_preview(orig: DynamicImage, preview_width: u32, preview_height: u32) -> DynamicImage {
    // FIX: distortion needs to be fixed
    let preview = orig.resize_exact(preview_width, preview_height, Triangle);
    return preview;
}

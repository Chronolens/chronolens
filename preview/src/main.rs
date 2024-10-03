use std::io::Cursor;
use std::str::from_utf8;
use std::sync::Arc;

use async_nats::jetstream::Message;
use async_nats::rustls::lock::Mutex;
use futures_util::StreamExt;
use image::{imageops::FilterType::Triangle, DynamicImage, ImageReader};
use s3::{creds::Credentials, error::S3Error, Bucket, BucketConfiguration, Region};
use uuid::Uuid;

// Flow to create a preview
// 1. Get event wiht uid of image from NATS
// 2. Get image from event, and fetch it form the object storage
// 3. Create image preview the image X
// 4. Save preview to object storage
// 5. Update entry in database

// TODO: create the server and client handling
// connect to object storage
// connect to database

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
    // connect to s3 storage
    let bucket_name = "preview";
    let region = Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: "http://localhost:9000".to_string(),
    };
    let credentials = Credentials::default().expect("Credentials died");

    // connect to nats
    let addrs = "placeholder.io";
    let stream_name = "previews".to_string();

    let client = match async_nats::connect(addrs).await {
        Ok(c) => c,
        Err(..) => panic!("Couldn't connect nats client."),
    };

    let jetstream = async_nats::jetstream::new(client);
    let stream = jetstream
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: stream_name,
            max_messages: 10000,
            ..Default::default()
        })
        .await?;

    let consumer = stream
        .get_or_create_consumer(
            "preview_consumer",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("preview_consumer".to_string()),
                ..Default::default()
            },
        )
        .await?;

    let bucket = match get_bucket(bucket_name, region, credentials).await {
        Ok(b) => b,
        Err(err) => panic!("Couldn't get preview bucket: {}", err),
    };

    let thread_bucket = Arc::new(Mutex::new(bucket));

    let mut messages = consumer.messages().await?;
    while let Some(message) = messages.next().await {
        match message {
            Ok(msg) => {
                println!(
                    "Message received: {:?}",
                    String::from_utf8(msg.payload.to_vec())
                );

                // TODO: spawn a thread for each event
                let thread_bucket_clone = thread_bucket.clone();
                tokio::spawn(async move { handle_message(msg, thread_bucket_clone).await });
            }
            Err(..) => {
                println!("Error receiving message");
                panic!();
            }
        }
    }
    Ok(())
}

async fn get_bucket(
    bucket_name: &str,
    region: Region,
    credentials: Credentials,
) -> Result<Box<Bucket>, S3Error> {
    let mut bucket = Bucket::new(bucket_name, region.clone(), credentials.clone())
        .expect("Couldn't create new bucket");

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
    return Ok(bucket);
}

// TODO: add orignal images bucket
// redo code and break it into functions
// test it
async fn handle_message(msg: Message, preview_bucket: Arc<Mutex<Box<Bucket>>>) {
    let payload_bytes: &[u8] = &msg.payload;
    let orig_image_id = match from_utf8(payload_bytes) {
        Ok(path) => path.to_owned(),
        Err(err) => {
            println!("Couldn't convert image path into utf8: {err:?}");
            return;
        }
    };

    // FIX: This bucket should be the original images bucket
    // lock bucket
    let pb_locked = match preview_bucket.lock() {
        Some(pb_locked) => pb_locked,
        _ => {
            println!("Bucket is poisoned in get object");
            return;
        }
    };

    let orig_image_response = match pb_locked.get_object(orig_image_id.clone()).await {
        Ok(oi) => oi,
        Err(err) => {
            println!("Get object failed: {err}");
            return;
        }
    };
    // FIX: check if dropping the lock here is correct
    drop(pb_locked);
    let orig_reader =
        match ImageReader::new(Cursor::new(orig_image_response.as_slice())).with_guessed_format() {
            Ok(rd) => rd,
            Err(err) => {
                println!("Couldn't convert image: {err}");
                return;
            }
        };
    let orig_image = match orig_reader.decode() {
        Ok(oi) => oi,
        Err(err) => {
            println!("Couldn't convert image: {err}");
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

    // FIX: lock bucket
    let preview_id = Uuid::new_v4().to_string();
    let pb_locked = match preview_bucket.lock() {
        Some(pb_locked) => pb_locked,
        _ => {
            println!("Bucket is poisoned in get object");
            return;
        }
    };
    let preview_response_data = match pb_locked.put_object(preview_id, &preview_bytes).await {
        Ok(rp) => rp,
        Err(err) => {
            println!("Put preview object failed with: {err}");
            return;
        }
    };
    if preview_response_data.status_code() != 200 {
        println!(
            "Put preview object failed with status code: {}",
            preview_response_data.status_code()
        );
        return;
    }
    // FIX: check here too
    drop(pb_locked);

    // TODO: db update

    match msg.ack().await {
        Ok(()) => (),
        Err(..) => println!("Couldn't acknowledge message"),
    }
}

fn create_preview(orig: DynamicImage, preview_width: u32, preview_height: u32) -> DynamicImage {
    // FIX: distortion needs to be fixed
    let preview = orig.resize_exact(preview_width, preview_height, Triangle);
    return preview;
}

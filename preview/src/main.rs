use std::error::Error;
use std::io::Cursor;
use std::str;
use std::sync::Arc;

use async_nats::jetstream::Message;
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
async fn main() -> Result<(), Box<dyn Error>> {
    // connect to nats
    let nats_addr = "localhost:4222";
    let stream_name = String::from("previews");

    let bucket_name = "chronolens";
    let bucket = setup_bucket(&bucket_name, "http://localhost:9000").await?;
    let client = match async_nats::connect(nats_addr).await {
        Ok(c) => c,
        Err(err) => panic!("Couldn't connect nats client.{err}"),
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

    let thread_bucket = Arc::new(bucket);
    let mut messages = consumer.messages().await?;
    while let Some(message) = messages.next().await {
        match message {
            Ok(msg) => {
                println!(
                    "Message received: {:?}",
                    String::from_utf8(msg.payload.to_vec())
                );

                // TODO: spawn a thread for each event
                let thread_bucket_clone = Arc::clone(&thread_bucket);
                tokio::spawn(async move { handle_message(msg, thread_bucket_clone).await });
            }
            Err(..) => {
                println!("Error receiving message");
                panic!();
            }
        }
    }
    return Ok(());
}

// TODO: add orignal images bucket
// redo code and break it into functions
// test it
async fn handle_message(msg: Message, preview_bucket: Arc<Box<Bucket>>) {
    let payload_bytes: &[u8] = &msg.payload;
    let orig_image_id = match str::from_utf8(payload_bytes) {
        Ok(path) => path.to_owned(),
        Err(err) => {
            println!("Couldn't convert image path into utf8: {err:?}");
            return;
        }
    };

    let orig_image_response = match preview_bucket.get_object(orig_image_id.clone()).await {
        Ok(oir) => oir,
        Err(err) => {
            println!("Get object failed: {err}");
            return;
        }
    };

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

    let preview_id = Uuid::new_v4().to_string();
    let preview_response_data = match preview_bucket.put_object(preview_id, &preview_bytes).await {
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

    // TODO: db update

    match msg.ack().await {
        Ok(()) => (),
        Err(..) => println!("Couldn't acknowledge message"),
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
    println!("yeye");
    return Ok(bucket);
}

fn create_preview(orig: DynamicImage, preview_width: u32, preview_height: u32) -> DynamicImage {
    // FIX: distortion needs to be fixed
    let preview = orig.resize_exact(preview_width, preview_height, Triangle);
    return preview;
}

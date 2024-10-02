use std::io::Cursor;

use async_nats::jetstream::Message;
use futures_util::StreamExt;
use image::{imageops::FilterType::Triangle, DynamicImage, ImageReader};
use s3::{creds::Credentials, Bucket, BucketConfiguration, Region};

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
    // let access_key = "GvvD9X0sWWrRGURPxeHS";
    // let secret_key = "fVVfN0xxXW1ffz1bsKKjyrKQB9OomX3djpMtSr8C";
    // let credentials = Credentials::new(Some(access_key), Some(secret_key), None, None, None)
    //     .expect("Couldn't create credentials");
    let credentials = Credentials::default().expect("Credentials died");
    let mut bucket = Bucket::new(bucket_name, region.clone(), credentials.clone())
        .expect("Couldn't create bucket");

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

    let test_path = "preview.jpg";
    let test_image = ImageReader::open(test_path)?.decode()?;
    // Convert image to bytes in jpg format
    let mut test_image_bytes: Vec<u8> = Vec::new();
    test_image.write_to(
        &mut Cursor::new(&mut test_image_bytes),
        image::ImageFormat::Jpeg,
    )?;

    let response_data = bucket.put_object(test_path, &test_image_bytes).await?;
    assert_eq!(response_data.status_code(), 200);



    // connect to nats
    // let addrs = "placeholder.io";
    // let stream_name = "previews".to_string();
    //
    // let client = match async_nats::connect(addrs).await {
    //     Ok(c) => c,
    //     Err(..) => panic!("Couldn't connect nats client."),
    // };
    //
    // let jetstream = async_nats::jetstream::new(client);
    // let stream = jetstream
    //     .get_or_create_stream(async_nats::jetstream::stream::Config {
    //         name: stream_name,
    //         max_messages: 10000,
    //         ..Default::default()
    //     })
    //     .await?;
    //
    // let consumer = stream
    //     .get_or_create_consumer(
    //         "preview_consumer",
    //         async_nats::jetstream::consumer::pull::Config {
    //             durable_name: Some("preview_consumer".to_string()),
    //             ..Default::default()
    //         },
    //     )
    //     .await?;
    //
    // let mut messages = consumer.messages().await?;
    // while let Some(message) = messages.next().await {
    //     match message {
    //         Ok(msg) => {
    //             println!(
    //                 "Message received: {:?}",
    //                 String::from_utf8(msg.payload.to_vec())
    //             );
    //
    //             // TODO: spawn a thread for each event
    //             handle_message(msg);
    //         }
    //         Err(..) => {
    //             println!("Error receiving message");
    //             panic!();
    //         }
    //     }
    // }
    Ok(())
}

async fn handle_message(msg: Message) {
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

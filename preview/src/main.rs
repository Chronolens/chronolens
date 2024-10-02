use async_nats::jetstream::Message;
use futures_util::StreamExt;
use image::{imageops::FilterType::Lanczos3, DynamicImage, ImageReader};

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

    let mut messages = consumer.messages().await?;
    while let Some(message) = messages.next().await {
        match message {
            Ok(msg) => {
                println!(
                    "Message received: {:?}",
                    String::from_utf8(msg.payload.to_vec())
                );

                // TODO: spawn a thread for each event
                handle_message(msg);
            }
            Err(..) => {
                println!("Error receiving message");
                panic!();
            }
        }
    }
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
    let preview = orig.resize_exact(preview_width, preview_height, Lanczos3);
    return preview;
}

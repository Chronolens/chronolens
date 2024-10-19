mod handler;
use database::DbManager;
use futures_util::StreamExt;
use handler::handle_request;
use log::{error, info};
use s3::{creds::Credentials, error::S3Error, Bucket, BucketConfiguration, Region};
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, Debug)]
pub struct EnvVars {
    #[serde(alias = "NATS_ENDPOINT")]
    #[serde(default = "nats_endpoint_default")]
    pub nats_endpoint: String,
    #[serde(alias = "OBJECT_STORAGE_ENDPOINT")]
    #[serde(default = "object_storage_endpoint_default")]
    pub object_storage_endpoint: String,
    #[serde(alias = "OBJECT_STORAGE_BUCKET")]
    pub object_storage_bucket: String,
    #[serde(alias = "OBJECT_STORAGE_REGION")]
    pub object_storage_region: String,
    #[serde(alias = "OBJECT_STORAGE_ACCESS_KEY")]
    pub object_storage_access_key: String,
    #[serde(alias = "OBJECT_STORAGE_SECRET_KEY")]
    pub object_storage_secret_key: String,
}

fn nats_endpoint_default() -> String {
    "http://localhost".to_string()
}
fn object_storage_endpoint_default() -> String {
    "http://localhost".to_string()
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let envs = match envy::from_env::<EnvVars>() {
        Ok(vars) => vars,
        Err(err) => panic!("{}", err),
    };

    let db = match DbManager::new().await {
        Ok(database) => database,
        Err(err) => panic!("{}", err),
    };

    let bucket = setup_bucket(&envs).await?;

    let client = match async_nats::connect(envs.nats_endpoint).await {
        Ok(c) => c,
        Err(err) => {
            panic!("Couldn't connect nats client: {err}");
        }
    };

    let jetstream = async_nats::jetstream::new(client);
    let stream_name = String::from("previews");
    let stream = jetstream
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: stream_name.clone(),
            max_messages: 10000,
            ..Default::default()
        })
        .await?;

    // FIX: crate a const or a env var for the preview consumer
    let consumer = stream
        .get_or_create_consumer(
            "preview_consumer",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("preview_consumer".to_string()),
                filter_subject: "previews".to_string(),
                ..Default::default()
            },
        )
        .await?;

    let messages = consumer.messages().await?;
    let thread_limit = 5;
    let _ = messages
        .for_each_concurrent(thread_limit, |msg| {
            let thread_bucket = bucket.clone();
            let thread_db = db.clone();
            async move {
                match msg {
                    Ok(msg) => {
                        info!(
                            "Message received: {:?}",
                            String::from_utf8(msg.payload.to_vec())
                        );
                        handle_request(msg, thread_bucket, thread_db).await
                    }
                    Err(err) => {
                        error!("Error receiving message: {err}");
                    }
                }
            }
        })
        .await;
    Ok(())
}

async fn setup_bucket(envs: &EnvVars) -> Result<Box<Bucket>, S3Error> {
    // connect to s3 storage
    let region_obj = Region::Custom {
        region: envs.object_storage_region.to_string(),
        endpoint: envs.object_storage_endpoint.to_string(),
    };
    let credentials = Credentials::new(
        Some(&envs.object_storage_access_key),
        Some(&envs.object_storage_secret_key),
        None,
        None,
        None,
    )?;

    let mut bucket = Bucket::new(
        &envs.object_storage_bucket,
        region_obj.clone(),
        credentials.clone(),
    )?
    .with_path_style();

    if !bucket.exists().await? {
        bucket = Bucket::create_with_path_style(
            &envs.object_storage_bucket,
            region_obj,
            credentials,
            BucketConfiguration::default(),
        )
        .await?
        .bucket;
    }
    Ok(bucket)
}

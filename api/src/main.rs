mod models;
mod routes;
mod utils;
use axum::{
    extract::DefaultBodyLimit,
    routing::{post, Router},
};
use database::DbManager;
use jsonwebtoken::EncodingKey;
use routes::{login::login, upload_image::upload_image};
use s3::{creds::Credentials, error::S3Error, Bucket, BucketConfiguration, Region};
use serde::Deserialize;

#[derive(Clone)]
pub struct ServerConfig {
    pub database: DbManager,
    pub secret: EncodingKey,
    pub bucket: Box<Bucket>,
}

#[derive(Deserialize, Debug)]
pub struct EnvVars {
    #[serde(alias = "LISTEN_ON")]
    #[serde(default = "listen_on_default")]
    pub listen_on: String,
    #[serde(alias = "JWT_SECRET")]
    pub jwt_secret: String,
    #[serde(alias = "OBJECT_STORAGE_ENDPOINT")]
    #[serde(default = "object_storage_endpoint_default")]
    pub object_storage_endpoint: String,
    #[serde(alias = "OBJECT_STORAGE_BUCKET")]
    pub object_storage_bucket: String,
    #[serde(alias = "OBJECT_STORAGE_ACCESS_KEY")]
    pub object_storage_access_key: String,
    #[serde(alias = "OBJECT_STORAGE_SECRET_KEY")]
    pub object_storage_secret_key: String,
}

fn listen_on_default() -> String {
    "0.0.0.0:8080".to_string()
}

fn object_storage_endpoint_default() -> String {
    "http://localhost".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let environment_variables = match envy::from_env::<EnvVars>() {
        Ok(vars) => vars,
        Err(err) => panic!("{}", err),
    };

    let database = match DbManager::new().await {
        Ok(database) => database,
        Err(err) => panic!("{}", err),
    };

    let secret = EncodingKey::from_secret(environment_variables.jwt_secret.as_ref());

    let bucket = match setup_bucket(&environment_variables)
    .await
    {
        Ok(bucket) => bucket,
        Err(err) => panic!("{}", err),
    };

    let server_config = ServerConfig {
        database,
        secret,
        bucket,
    };
    // build our application with a route
    let app = Router::new()
        .route("/login", post(login))
        .route(
            "/image/upload",
            post(upload_image).route_layer(DefaultBodyLimit::max(10737418240)),
        )
        .with_state(server_config);

    let listener = tokio::net::TcpListener::bind(&environment_variables.listen_on)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn setup_bucket(envs: &EnvVars) -> Result<Box<Bucket>, S3Error> {
    // connect to s3 storage
    let region = Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: envs.object_storage_endpoint.to_string(),
    };
    // INFO: this credentials are fetched from the default location of the aws
    // credentials (~/.aws/credentials)
    //let credentials = Credentials::default().expect("Credentials died");
    let credentials = Credentials::new(Some(&envs.object_storage_access_key), Some(&envs.object_storage_secret_key), None, None, None)?;

    let mut bucket =
        Bucket::new(&envs.object_storage_bucket, region.clone(), credentials.clone())?.with_path_style();

    if !bucket.exists().await? {
        bucket = Bucket::create_with_path_style(
            &envs.object_storage_bucket,
            region,
            credentials,
            BucketConfiguration::default(),
        )
        .await?
        .bucket;
    }
    Ok(bucket)
}

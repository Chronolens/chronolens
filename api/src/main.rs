mod models;
mod routes;
mod utils;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, Router},
};
use chrono::Utc;
use database::DbManager;
use http::StatusCode;
use jsonwebtoken::{decode, DecodingKey, Validation};
use models::api_models::TokenClaims;
use routes::{
    login::login, preview::preview, sync_full::sync_full, upload_image::upload_image
};
use s3::{creds::Credentials, error::S3Error, Bucket, BucketConfiguration, Region};
use serde::Deserialize;

#[derive(Clone)]
pub struct ServerConfig {
    pub database: DbManager,
    pub secret: String,
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

    let secret = environment_variables.jwt_secret.clone();

    let bucket = match setup_bucket(&environment_variables).await {
        Ok(bucket) => bucket,
        Err(err) => panic!("{}", err),
    };

    let server_config = ServerConfig {
        database,
        secret,
        bucket,
    };
    // build our application with a route
    let public_routes = Router::new().route("/login", post(login));

    let private_routes = Router::new()
        .route(
            "/image/upload",
            post(upload_image).route_layer(DefaultBodyLimit::max(10737418240)),
        )
        .route("/sync/full", get(sync_full))
        .route("/preview/:media_id", get(preview))
        .layer(middleware::from_fn_with_state(
            server_config.secret.clone(),
            auth_middleware,
        ));

    let listener = tokio::net::TcpListener::bind(&environment_variables.listen_on)
        .await
        .unwrap();

    let app = public_routes
        .merge(private_routes)
        .with_state(server_config);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn setup_bucket(envs: &EnvVars) -> Result<Box<Bucket>, S3Error> {
    // connect to s3 storage
    let region = Region::Custom {
        region: "eu-central-1".to_string(),
        endpoint: envs.object_storage_endpoint.to_string(),
    };
    // INFO: these credentials are fetched from the default location of the aws
    // credentials (~/.aws/credentials)
    //let credentials = Credentials::default().expect("Credentials died");
    let credentials = Credentials::new(
        Some(&envs.object_storage_access_key),
        Some(&envs.object_storage_secret_key),
        None,
        None,
        None,
    )?;

    let mut bucket = Bucket::new(
        &envs.object_storage_bucket,
        region.clone(),
        credentials.clone(),
    )?
    .with_path_style();

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

async fn auth_middleware(
    State(secret): State<String>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let authorization_header = match req.headers_mut().get(http::header::AUTHORIZATION) {
        Some(header) => header,
        None => return (StatusCode::UNAUTHORIZED, "No authorization header found").into_response(),
    };
    let mut authorization_header_str = match authorization_header.to_str() {
        Ok(token) => token.split_whitespace(),
        Err(..) => {
            return (StatusCode::UNAUTHORIZED, "Authorization header is empty").into_response()
        }
    };

    let (_, jwt_header) = (
        authorization_header_str.next(),
        authorization_header_str.next(),
    );

    let secret = &DecodingKey::from_secret(secret.as_ref());

    let result = match decode::<TokenClaims>(
        jwt_header.unwrap(),
        secret,
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    ) {
        Ok(token) => token,
        Err(err) => {
            return (
                StatusCode::UNAUTHORIZED,
                format!("Could not decode JWT token {}", err),
            )
                .into_response()
        }
    };

    let now = Utc::now().timestamp_millis();
    if now < result.claims.iat || now > result.claims.exp {
        return (StatusCode::UNAUTHORIZED, "Authorization header is invalid").into_response();
    }

    req.extensions_mut().insert(result.claims.user_id);

    next.run(req).await
}

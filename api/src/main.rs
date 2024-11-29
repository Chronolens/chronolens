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
use models::api_models::AccessTokenClaims;
use routes::{
    clip_search::clip_search, cluster_previews::cluster_previews, create_face::create_face, face_previews::face_previews, faces::faces, login::login, logs::logs, media::media, preview::preview, previews::previews, refresh::refresh, register::register, sync_full::sync_full, sync_partial::sync_partial, upload_image::upload_image

};
use s3::{creds::Credentials, error::S3Error, Bucket, BucketConfiguration, Region};
use serde::Deserialize;

#[derive(Clone)]
pub struct ServerConfig {
    pub database: DbManager,
    pub secret: String,
    pub bucket: Box<Bucket>,
    pub nats_jetstream: async_nats::jetstream::Context,
    pub nats_client: async_nats::Client,
}

#[derive(Deserialize, Debug)]
pub struct EnvVars {
    #[serde(alias = "LISTEN_ON")]
    #[serde(default = "listen_on_default")]
    pub listen_on: String,
    #[serde(alias = "JWT_SECRET")]
    pub jwt_secret: String,
    #[serde(alias = "NATS_ENDPOINT")]
    #[serde(default = "nats_endpoint_default")]
    pub nats_endpoint: String,
    #[serde(alias = "OBJECT_STORAGE_ENDPOINT")]
    #[serde(default = "object_storage_endpoint_default")]
    pub object_storage_endpoint: String,
    #[serde(alias = "OBJECT_STORAGE_BUCKET")]
    pub object_storage_bucket: String,
    #[serde(alias = "OBJECT_STORAGE_ACCESS_KEY")]
    pub object_storage_access_key: String,
    #[serde(alias = "OBJECT_STORAGE_SECRET_KEY")]
    pub object_storage_secret_key: String,
    #[serde(alias = "OBJECT_STORAGE_REGION")]
    pub object_storage_region: String,
}

fn listen_on_default() -> String {
    "0.0.0.0:8080".to_string()
}

fn nats_endpoint_default() -> String {
    "http://localhost".to_string()
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

    let nats_client = match async_nats::connect(environment_variables.nats_endpoint.clone()).await {
        Ok(c) => c,
        Err(err) => {
            panic!("Couldn't connect to NATS client: {err}");
        }
    };

    let nats_jetstream = async_nats::jetstream::new(nats_client.clone());

    let server_config = ServerConfig {
        database,
        secret,
        bucket,
        nats_jetstream,
        nats_client,
    };

    let public_routes = Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/refresh", post(refresh));

        let private_routes = Router::new()
        .route(
            "/image/upload",
            post(upload_image).route_layer(DefaultBodyLimit::max(10737418240)),
        )
        .route("/sync/full", get(sync_full))
        .route("/sync/partial", get(sync_partial))
        .route("/previews", get(previews))
        .route("/preview/:media_id", get(preview))
        .route("/media/:media_id", get(media))
        .route("/logs", get(logs))
        .route("/faces", get(faces))
        .route("/cluster/:cluster_id", get(cluster_previews))
        .route("/face/:face_id", get(face_previews))
        .route("/search", get(clip_search))
        .route("/create_face", post(create_face)) 
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

    let (bearer_keyword, jwt_header) = (
        authorization_header_str.next(),
        authorization_header_str.next(),
    );

    if bearer_keyword != Some("Bearer") {
        return (
            StatusCode::UNAUTHORIZED,
            "Authorization header must contain a Bearer token",
        )
            .into_response();
    }

    let secret = &DecodingKey::from_secret(secret.as_ref());

    let result = match decode::<AccessTokenClaims>(
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

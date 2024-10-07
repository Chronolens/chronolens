mod db;
mod models;
mod nats_api;
mod routes;
use axum::{extract::DefaultBodyLimit, routing::{post, Router}};
use db::DbAccess;
use jsonwebtoken::EncodingKey;
use models::server_models::{EnvVars, ServerConfig};
use routes::{login::login, upload_image::upload_image};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let environment_variables = match envy::from_env::<EnvVars>() {
        Ok(vars) => vars,
        Err(err) => panic!("{}", err),
    };

    let database = match DbAccess::new().await {
        Ok(database) => database,
        Err(err) => panic!("{}", err),
    };

    let secret = EncodingKey::from_secret(environment_variables.jwt_secret.as_ref());

    let server_config = ServerConfig { database, secret };
    // build our application with a route
    let app = Router::new()
        .route("/login", post(login))
        .route("/image/upload", post(upload_image).route_layer(DefaultBodyLimit::max(10737418240)))
        .with_state(server_config);

    let listener = tokio::net::TcpListener::bind(environment_variables.listen_on)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

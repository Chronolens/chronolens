use jsonwebtoken::EncodingKey;
use serde::Deserialize;
use crate::DbManager;

#[derive(Clone)]
pub struct ServerConfig {
    pub database: DbManager,
    pub secret: EncodingKey,
}

#[derive(Deserialize, Debug)]
pub struct EnvVars {
    #[serde(alias = "LISTEN_ON")]
    #[serde(default = "listen_on_default")]
    pub listen_on: String,
    #[serde(alias = "JWT_SECRET")]
    pub jwt_secret: String,
}

fn listen_on_default() -> String {
    "0.0.0.0:8080".to_string()
}

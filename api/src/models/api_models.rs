use database::{RemoteMediaAdded, RemoteMediaDeleted};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub iat: i64,
    pub exp: i64,
    pub user_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub iat: i64,
    pub exp: i64,
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

#[derive(Serialize)]
pub struct PartialSyncResponse {
    pub uploaded: Vec<RemoteMediaAdded>,
    pub deleted: Vec<RemoteMediaDeleted>,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct PreviewResponse {
    pub previews: Vec<String>,
}

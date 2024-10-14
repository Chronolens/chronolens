use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize)]
pub struct TokenClaims {
    pub iat: i64,
    pub exp: i64,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct MediaInfoResponse {
    pub hash: String,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct PartialSyncResponse {
    pub uploaded: HashMap<String,MediaInfoResponse>,
    pub deleted: Vec<String>
}

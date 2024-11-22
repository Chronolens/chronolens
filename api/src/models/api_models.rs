use std::collections::HashMap;

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
    pub previews: HashMap<String, String>,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ClusterPreviewResponse {
    pub previews: Vec<HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct GetFacesResponse {
    pub faces: Vec<FaceResponse>,
    pub clusters: Vec<ClusterResponse>,
}

#[derive(Serialize)]
pub struct FaceResponse {
    pub face_id: i32,
    pub name: String,
    pub photo_url: String,
    pub bbox: Vec<i32>,
}

#[derive(Serialize)]
pub struct ClusterResponse {
    pub cluster_id: i32,
    pub photo_url: String,
    pub bbox: Vec<i32>,
}

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreviewItem {
    pub id: String,
    pub preview_url: String,
}

#[derive(Serialize)]
pub struct MediaMetadataResponse {
    pub id: String,
    pub media_url: String,
    pub created_at: i64,
    pub file_size: i64,
    pub file_name: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub image_width: Option<i32>,
    pub image_length: Option<i32>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub fnumber: Option<String>,
    pub exposure_time: Option<String>,
    pub photographic_sensitivity: Option<String>,
    pub orientation: Option<i32>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

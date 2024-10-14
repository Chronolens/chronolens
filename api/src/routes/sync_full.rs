use std::collections::HashMap;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::StatusCode;

use crate::{models::api_models::MediaInfoResponse, ServerConfig};

pub async fn sync_full(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
) -> Response {
    let remote_media = match server_config.database.sync_full(user_id).await {
        Ok(media) => media,
        Err(..) => return (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    };

    let mut sync_full_response: HashMap<String, MediaInfoResponse> = HashMap::new();

    remote_media.into_iter().for_each(|media| {
        sync_full_response.insert(
            media.id,
            MediaInfoResponse {
                hash: media.hash,
                created_at: media.created_at,
            },
        );
    });

    (StatusCode::OK, Json(sync_full_response)).into_response()
}

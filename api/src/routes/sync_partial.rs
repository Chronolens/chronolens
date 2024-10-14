use std::collections::HashMap;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::{HeaderMap, StatusCode};

use crate::{
    models::api_models::{MediaInfoResponse, PartialSyncResponse},
    ServerConfig,
};

pub async fn sync_partial(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    headers: HeaderMap,
) -> Response {
    let since = match headers.get("Since").and_then(|ct| ct.to_str().ok()) {
        Some(since) => match since.parse::<i64>() {
            Ok(since) => since,
            Err(..) => {
                return (StatusCode::BAD_REQUEST, "Since header could not be decoded")
                    .into_response()
            }
        },
        None => return (StatusCode::BAD_REQUEST, "Since header does not exist").into_response(),
    };

    let (media_uploaded, media_deleted) =
        match server_config.database.sync_partial(user_id, since).await {
            Ok(media) => media,
            Err(..) => return (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        };

    let mut media_uploaded_map: HashMap<String, MediaInfoResponse> = HashMap::new();
    let mut media_deleted_map: Vec<String> = vec![];

    // Step 3: Populate media_added_map
    media_uploaded.into_iter().for_each(|media| {
        media_uploaded_map.insert(
            media.id.clone(), // Assuming media.hash is the key
            MediaInfoResponse {
                hash: media.hash.clone(),
                created_at: media.created_at,
            },
        );
    });

    // Step 4: Populate media_deleted_map
    media_deleted.into_iter().for_each(|media| {
        media_deleted_map.push(media.id);
    });


    let response = PartialSyncResponse {
        uploaded: media_uploaded_map,
        deleted: media_deleted_map
    };

    (StatusCode::OK, Json(response)).into_response()
}

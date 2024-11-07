use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::Utc;
use http::{HeaderMap, StatusCode};

use crate::{models::api_models::PartialSyncResponse, ServerConfig};

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

    let mut headers = HeaderMap::new();
    headers.insert("Since", Utc::now().timestamp_millis().into()); // Add your headers here

    let response = PartialSyncResponse {
        uploaded: media_uploaded,
        deleted: media_deleted,
    };

    // Build the response with the headers and the JSON body
    (StatusCode::OK, headers, Json(response)).into_response()
}

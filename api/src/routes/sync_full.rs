use axum::{extract::State, response::{IntoResponse, Response}, Extension, Json};
use chrono::Utc;
use http::{HeaderMap, StatusCode};

use crate::ServerConfig;

pub async fn sync_full(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
) -> Response {
    let remote_media = match server_config.database.sync_full(user_id).await {
        Ok(media) => media,
        Err(..) => return (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    };

    let mut headers = HeaderMap::new();
    // TODO: Get the max here and on the partial aswell

    //let timestamp = remote_media.iter().max_by(|&a,&b| a.created_at.cmp(&b.created_at));
    headers.insert("Since", Utc::now().timestamp_millis().into()); // Add your headers here

    // Build the response with the headers and the JSON body
    (StatusCode::OK, headers, Json(remote_media)).into_response()
}

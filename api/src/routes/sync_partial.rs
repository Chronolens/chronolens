use std::{collections::HashMap, i64};

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::{HeaderMap, StatusCode};

use crate::{models::api_models::SyncResponse, ServerConfig};

pub async fn sync_partial(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    headers: HeaderMap,
) -> Response {

    
    let since = match headers.get("Since").and_then(|ct| ct.to_str().ok()) {
        Some(since) => match since.parse::<i64>(){
            Ok(since) => since,
            Err(..) => 
            return (
                StatusCode::BAD_REQUEST,
                "Since header could not be decoded"
            )
                .into_response()

        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Since header does not exist",
            )
                .into_response()
        }
    };

    let remote_media = match server_config.database.sync_partial(user_id,since).await {
        Ok(media) => media,
        Err(..) => return (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    };

    let mut partial_full_response: HashMap<String, SyncResponse> = HashMap::new();

    remote_media.into_iter().for_each(|media| {
        partial_full_response.insert(
            media.hash,
            SyncResponse {
                id: media.id,
                created_at: media.created_at,
            },
        );
    });

    (StatusCode::OK, Json(partial_full_response)).into_response()
}

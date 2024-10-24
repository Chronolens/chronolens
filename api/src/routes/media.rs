use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Extension,
};
use http::StatusCode;

use crate::ServerConfig;

pub async fn media(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Path(media_id): Path<String>,
) -> Response {
    let user_has_media = match server_config
        .database
        .user_has_media(user_id, &media_id)
        .await
    {
        Ok(has_media) => has_media,
        Err(..) => {
            return (
                StatusCode::UNAUTHORIZED,
                "Media does not exist or user does not have permissions to access it",
            )
                .into_response()
        }
    };

    if user_has_media {
        let url = server_config
            .bucket
            .presign_get(media_id, 86400, None)
            .await
            .unwrap();
        (StatusCode::OK, url).into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Media does not exist or user does not have permissions to access it",
        )
            .into_response()
    }
}

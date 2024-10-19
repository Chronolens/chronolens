use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Extension,
};
use database::GetPreviewError;
use http::StatusCode;

use crate::ServerConfig;

pub async fn preview(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Path(media_id): Path<String>,
) -> Response {
    match server_config
        .database
        .get_preview_from_user(user_id, &media_id)
        .await
    {
        Ok(preview_id) => {
            let url = server_config
                .bucket
                .presign_get(preview_id, 86400, None)
                .await
                .unwrap();
            (StatusCode::OK, url).into_response()
        }
        Err(GetPreviewError::InternalError) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        Err(GetPreviewError::NotFound) => (
            StatusCode::UNAUTHORIZED,
            "Media does not exist or user does not have permissions to access it",
        )
            .into_response(),
    }
}

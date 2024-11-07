use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use database::GetPreviewError;
use futures_util::future::join_all;
use http::StatusCode;
use serde::Deserialize;

use crate::{models::api_models::PreviewResponse, ServerConfig};

#[derive(Deserialize)]
pub struct Pagination {
    page: Option<u64>,
    page_size: Option<u64>,
}

pub async fn previews(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Query(pagination): Query<Pagination>,
) -> Response {
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(10).clamp(1, 30);

    match server_config
        .database
        .get_previews(user_id, page, page_size)
        .await
    {
        Ok(preview_ids) => {
            // Generate URLs for each preview ID
            let urls = join_all(
                preview_ids
                    .into_iter()
                    .map(|preview_id| server_config.bucket.presign_get(preview_id, 86400, None)),
            )
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<String>>();

            // Return the list of URLs as JSON
            (StatusCode::OK, Json(PreviewResponse { previews: urls })).into_response()
        }
        Err(GetPreviewError::InternalError) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        Err(GetPreviewError::NotFound) => (
            StatusCode::UNAUTHORIZED,
            "Media does not exist or user does not have permissions to access it",
        )
            .into_response(),
    }
}

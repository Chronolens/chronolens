use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use database::GetPreviewError;
use http::StatusCode;

use crate::{
    models::api_models::{Pagination, PreviewItem},
    ServerConfig,
};

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
            let previews: Vec<PreviewItem> = futures_util::future::join_all(
                preview_ids.into_iter().map(|(media_id, preview_id)| {
                    let bucket = server_config.bucket.clone();
                    async move {
                        if let Some(p_id) = preview_id {
                            match bucket.presign_get(p_id, 86400, None).await {
                                Ok(url) => Some(PreviewItem {
                                    id: media_id,
                                    preview_url: url,
                                }),
                                Err(_) => None,
                            }
                        } else {
                            Some(PreviewItem {
                                id: media_id,
                                preview_url: "".to_string(),
                            })
                        }
                    }
                }),
            )
            .await
            .into_iter()
            .flatten()
            .collect();
            (StatusCode::OK, Json(previews)).into_response()
        }
        Err(GetPreviewError::InternalError) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        Err(GetPreviewError::NotFound) => (
            StatusCode::FORBIDDEN,
            "Cluster does not exist or user does not have permissions to access it",
        )
            .into_response(),
    }
}

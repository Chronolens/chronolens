use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use database::GetPreviewError;
use http::StatusCode;
use serde::Deserialize;

use crate::{models::api_models::PreviewItem, ServerConfig};

#[derive(Deserialize)]
pub struct ClusterPagination {
    page: Option<u64>,
    page_size: Option<u64>,
}

pub async fn cluster_previews(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Path(cluster_id): Path<i32>,
    Query(params): Query<ClusterPagination>,
) -> Response {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 30);

    match server_config
        .database
        .get_cluster_previews(user_id.clone(), cluster_id, page, page_size)
        .await
    {
        Ok(preview_ids) => {
            let previews: Vec<PreviewItem> = futures_util::future::join_all(
                preview_ids.into_iter().map(|(media_id, preview_id)| {
                    let bucket = server_config.bucket.clone();
                    async move {
                        match bucket.presign_get(preview_id, 86400, None).await {
                            Ok(url) => Some(PreviewItem {
                                id: media_id,
                                preview_link: url,
                            }),
                            Err(_) => None,
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
            StatusCode::UNAUTHORIZED,
            "Cluster does not exist or user does not have permissions to access it",
        )
            .into_response(),
    }
}

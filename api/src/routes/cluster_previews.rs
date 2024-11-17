use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::StatusCode;
use serde::Deserialize;

use crate::{models::api_models::PreviewResponse, ServerConfig};

#[derive(Deserialize)]
pub struct ClusterPagination {
    page: Option<u64>,
    page_size: Option<u64>,
    cluster_id: String,
}

pub async fn cluster_previews(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Query(params): Query<ClusterPagination>,
) -> Response {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 30);

    match server_config
        .database
        .get_cluster_previews(user_id.clone(), params.cluster_id.clone(), page, page_size)
        .await
    {
        Ok(preview_ids) => {
            let urls: HashMap<String, String> =
                futures_util::future::join_all(preview_ids.into_iter().map(|(media_id, preview_id)| {
                    let bucket = server_config.bucket.clone();
                    async move {
                        match bucket.presign_get(preview_id, 86400, None).await {
                            Ok(url) => Some((media_id, url)), 
                            Err(_) => None,                   
                        }
                    }
                }))
                .await
                .into_iter()
                .flatten()
                .collect::<HashMap<String, String>>();

            (StatusCode::OK, Json(PreviewResponse { previews: urls })).into_response()
        }
        Err(crate::database::GetPreviewError::InternalError) => {
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
        Err(crate::database::GetPreviewError::NotFound) => (
            StatusCode::UNAUTHORIZED,
            "Cluster does not exist or user does not have permissions to access it",
        )
            .into_response(),
    }
}

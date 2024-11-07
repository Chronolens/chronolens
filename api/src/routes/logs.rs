use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json, Response},
    Extension,
};
use database::{GetLogError, LogResponse};
use http::StatusCode;
use serde::Deserialize;

use crate::ServerConfig;

#[derive(Deserialize)]
pub struct Pagination {
    page: Option<u64>,
    page_size: Option<u64>,
}

pub async fn logs(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Query(pagination): Query<Pagination>,
) -> Response {
    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(10).clamp(1, 30);

    match server_config.database.get_logs(user_id, page, page_size).await {
        Ok(log_entries) => {
            (StatusCode::OK, Json(LogResponse { logs: log_entries })).into_response()
        }
        Err(GetLogError::InternalError) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        Err(GetLogError::NotFound) => (StatusCode::NOT_FOUND).into_response(),
    }
}

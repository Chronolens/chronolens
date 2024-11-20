use axum::{
    extract::{Query, State, Extension},
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use tokio_util::bytes::Bytes; 
use serde_json::json; 
use crate::{ServerConfig, models::api_models::{PreviewItem, SearchQuery}};

pub async fn clip_search(
    State(server_config): State<ServerConfig>, 
    Extension(user_id): Extension<String>,    
    Query(params): Query<SearchQuery>,        
) -> Response {
    let query = params.query;
    let page = params.page.unwrap_or(1).max(1); 
    let page_size = params.page_size.unwrap_or(10).clamp(1, 30); 

    if query.is_empty() {
        return (StatusCode::BAD_REQUEST, "Query is required").into_response(); 
    }

    let message = json!({
        "user_id": user_id,
        "query": query,
    });

    let message_bytes = Bytes::from(serde_json::to_vec(&message).unwrap());

    match server_config
        .nats_jetstream
        .publish("clip-process", message_bytes) 
        .await
    {
        Ok(_) => (), 
        Err(..) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    (StatusCode::OK, Json("Query sent successfully")).into_response()
}

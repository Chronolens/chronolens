use crate::{
    models::api_models::{PreviewItem, SearchQuery},
    ServerConfig,
};
use async_nats::Client;
use axum::{
    extract::{Extension, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde_json::json;

pub async fn clip_search(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Query(params): Query<SearchQuery>,
) -> Response {
    let query = params.query;
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 30);

    // println!("Received request for clip_search with query: {}, page: {}, pagesize: {}", query, page, page_size);

    if query.is_empty() {
        // println!("Query is empty");
        return (StatusCode::BAD_REQUEST, "Query is required").into_response();
    }

    let request_message = json!({
        "user_id": user_id,
        "query": query,
        "page": page,
        "page_size": page_size,
    });

    let nats_client: &Client = &server_config.nats_client;

    let subject = "clip-process-search";
    let response = match nats_client
        .request(subject, request_message.to_string().into())
        .await
    {
        Ok(response) => response,
        Err(_) => {
            // eprintln!("Failed to send request to NATS: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error sending request").into_response();
        }
    };

    let response_data = match String::from_utf8(response.payload.to_vec()) {
        Ok(data) => data,
        Err(_) => {
            // eprintln!("Failed to parse NATS response: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error parsing response").into_response();
        }
    };

    let preview_items: Result<Vec<PreviewItem>, _> = serde_json::from_str(&response_data);

    match preview_items {
        Ok(items) => Json(json!(items)).into_response(),
        Err(_) => {
            // eprintln!("Failed to deserialize response data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error deserializing response",
            )
                .into_response()
        }
    }
}

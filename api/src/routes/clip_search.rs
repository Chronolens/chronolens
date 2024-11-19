use axum::{
    extract::{Query, State, Extension},
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
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
   
    let response_data = vec![
        PreviewItem {
            id: format!("item-{}-1", page),
            preview_url: format!("https://example.com/preview/{}-1", page),
        },
        PreviewItem {
            id: format!("item-{}-2", page),
            preview_url: format!("https://example.com/preview/{}-2", page),
        },
    ];
   
    (StatusCode::OK, Json(response_data)).into_response()
}

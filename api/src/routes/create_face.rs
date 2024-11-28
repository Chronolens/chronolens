use axum::{
    extract::{Json, State, Extension},
    http::StatusCode,
};
use crate::models::api_models::CreateFacePayload;
use crate::ServerConfig;

pub async fn create_face(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateFacePayload>,
) -> StatusCode {
    // println!("User ID: {}", user_id);
    // println!("Payload IDs: {:?}", payload.ids);
    // println!("Payload Name: {}", payload.name);
    match server_config
        .database
        .insert_face(user_id.clone(), payload.ids.clone(), payload.name.clone())
        .await
    {
        Ok(_) => StatusCode::OK,                
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR, 
    }
}

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use database::AddUserError;
use http::StatusCode;

use crate::{models::api_models::RegisterRequest, ServerConfig};

pub async fn register(
    State(server_config): State<ServerConfig>,
    Json(register_request): Json<RegisterRequest>,
) -> Response {
    let uuid = uuid::Uuid::new_v4();

    let password_hashed = match bcrypt::hash(register_request.password, 12) {
        Ok(pw) => pw,
        Err(..) => return (StatusCode::FORBIDDEN).into_response(),
    };

    match server_config
        .database
        .add_user(uuid.to_string(), register_request.username, password_hashed)
        .await
    {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(AddUserError::AlreadyExists) => (
            StatusCode::FORBIDDEN,
            "A user with that username already exists",
        )
            .into_response(),
        Err(AddUserError::InternalError) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    }
}

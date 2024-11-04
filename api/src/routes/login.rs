use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use http::StatusCode;

use crate::{
    models::api_models::{LoginRequest, TokenResponse},
    utils::jwt::generate_jwt_tokens,
    ServerConfig,
};

pub async fn login(
    State(server_config): State<ServerConfig>,
    Json(login_request): Json<LoginRequest>,
) -> Response {
    let user = match server_config
        .database
        .get_user(login_request.username.clone())
        .await
    {
        Ok(pw) => pw,
        Err(..) => return (StatusCode::FORBIDDEN).into_response(),
    };

    let matched = match bcrypt::verify(login_request.password, &user.password) {
        Ok(matched) => matched,
        Err(..) => return (StatusCode::FORBIDDEN).into_response(),
    };

    if matched {
        let now = Utc::now().timestamp_millis();
        let access_expires_at = now + 3_600_000;
        let refresh_expires_at = now + 172_800_000;

        let (access_token, refresh_token) = match generate_jwt_tokens(
            server_config.secret,
            user.id,
            now,
            access_expires_at,
            refresh_expires_at,
        ) {
            Ok(tokens) => tokens,
            Err(..) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error generating JWT tokens",
                )
                    .into_response()
            }
        };
        (
            StatusCode::OK,
            Json(TokenResponse {
                access_token,
                refresh_token,
                expires_at: access_expires_at,
            }),
        )
            .into_response()
    } else {
        (StatusCode::FORBIDDEN).into_response()
    }
}

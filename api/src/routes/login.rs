use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::Header;

use crate::models::{
    api_models::{LoginRequest, LoginResponse},
    server_models::ServerConfig,
};

pub async fn login(
    State(server_config): State<ServerConfig>,
    Json(login_request): Json<LoginRequest>,
) -> Response {
    let password_hash = match server_config
        .database
        .get_user_password(login_request.username)
        .await
    {
        Ok(pw) => pw.password,
        Err(..) => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let matched = match bcrypt::verify(login_request.password, &password_hash) {
        Ok(matched) => matched,
        Err(..) => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    if matched {
        #[derive(serde::Serialize)]
        struct Claims {
            iat: i64,
            nbf: i64,
        }
        let claims = Claims {
            iat: chrono::offset::Local::now().timestamp_millis(),
            nbf: chrono::offset::Local::now().timestamp_millis() + 604_800_000,
        };

        let token = match jsonwebtoken::encode(&Header::default(), &claims, &server_config.secret) {
            Ok(token) => token,
            Err(..) => panic!("Error generating JWT token"),
        };
        (StatusCode::OK, Json(LoginResponse { token })).into_response()
    } else {
        (StatusCode::UNAUTHORIZED).into_response()
    }
}

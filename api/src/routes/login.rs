use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};

use crate::{
    models::api_models::{LoginRequest, LoginResponse, TokenClaims},
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
        Err(..) => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    let matched = match bcrypt::verify(login_request.password, &user.password) {
        Ok(matched) => matched,
        Err(..) => return (StatusCode::UNAUTHORIZED).into_response(),
    };

    if matched {
        let claims = TokenClaims {
            iat: Utc::now().timestamp_millis(),
            exp: Utc::now().timestamp_millis() + 604_800_000,
            user_id: user.id.clone()
        };

        let secret = &EncodingKey::from_secret(server_config.secret.as_ref());

        let token = match jsonwebtoken::encode(&Header::default(), &claims, secret) {
            Ok(token) => token,
            Err(..) => panic!("Error generating JWT token"),
        };
        (StatusCode::OK, Json(LoginResponse { token })).into_response()
    } else {
        (StatusCode::UNAUTHORIZED).into_response()
    }
}

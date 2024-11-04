use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use http::StatusCode;
use jsonwebtoken::{DecodingKey, Validation};

use crate::{
    models::api_models::{
        AccessTokenClaims, RefreshTokenClaims, RefreshTokenRequest, TokenResponse,
    },
    utils::jwt::generate_jwt_tokens,
    ServerConfig,
};

pub async fn refresh(
    State(server_config): State<ServerConfig>,
    Json(refresh_token_request): Json<RefreshTokenRequest>,
) -> Response {
    let secret = &DecodingKey::from_secret(server_config.secret.as_ref());

    let mut now = Utc::now().timestamp_millis();
    //Decode the refresh token
    let refresh_token_data = match jsonwebtoken::decode::<RefreshTokenClaims>(
        &refresh_token_request.refresh_token,
        secret,
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    ) {
        Ok(token) => token,
        Err(..) => {
            return (StatusCode::BAD_REQUEST, "Could not decode JWT access token").into_response()
        }
    };

    if now < refresh_token_data.claims.iat || now > refresh_token_data.claims.exp {
        return (
            StatusCode::FORBIDDEN,
            "Refresh token is invalid, please reauthenticate",
        )
            .into_response();
    }

    if refresh_token_data.claims.access_token != refresh_token_request.access_token {
        return (
            StatusCode::FORBIDDEN,
            "The provided refresh token is not associated with the access_token",
        )
            .into_response();
    }

    let access_token_data = match jsonwebtoken::decode::<AccessTokenClaims>(
        &refresh_token_request.access_token,
        secret,
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    ) {
        Ok(token) => token,
        Err(..) => {
            return (
                StatusCode::BAD_REQUEST,
                "Could not decode JWT refresh token",
            )
                .into_response()
        }
    };

    now = Utc::now().timestamp_millis();
    let access_expires_at = now + 3_600_000;
    let refresh_expires_at = now + 172_800_000;
    let (new_access_token, new_refresh_token) = match generate_jwt_tokens(
        server_config.secret,
        access_token_data.claims.user_id,
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
            access_token: new_access_token,
            refresh_token: new_refresh_token,
            expires_at: access_expires_at,
        }),
    )
        .into_response()
}

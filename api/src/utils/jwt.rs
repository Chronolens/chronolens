use jsonwebtoken::{EncodingKey, Header};

use crate::{models::api_models::RefreshTokenClaims, AccessTokenClaims};

pub fn generate_jwt_tokens(
    secret: String,
    user_id: String,
    issued_at: i64,
    access_expires_at: i64,
    refresh_expires_at: i64,
) -> Result<(String, String), String> {
    let encoding_key = &EncodingKey::from_secret(secret.as_ref());

    let access_claims = AccessTokenClaims {
        iat: issued_at,
        exp: access_expires_at,
        user_id,
    };
    let access_token = match jsonwebtoken::encode(&Header::default(), &access_claims, encoding_key)
    {
        Ok(atoken) => atoken,
        Err(..) => return Err("Failed to create token".to_string()),
    };

    let refresh_claims = RefreshTokenClaims {
        iat: issued_at,
        exp: refresh_expires_at,
        access_token: access_token.clone(),
    };
    let refresh_token =
        match jsonwebtoken::encode(&Header::default(), &refresh_claims, encoding_key) {
            Ok(rtoken) => rtoken,
            Err(..) => return Err("Failed to create token".to_string()),
        };
    Ok((access_token, refresh_token))
}

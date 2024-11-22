use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::StatusCode;

use crate::{models::api_models::MediaMetadataResponse, ServerConfig};

pub async fn media(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    Path(media_id): Path<String>,
) -> Response {
    let user_has_media = match server_config
        .database
        .user_has_media(user_id, &media_id)
        .await
    {
        Ok(has_media) => has_media,
        Err(..) => {
            return (
                StatusCode::FORBIDDEN,
                "Media does not exist or user does not have permissions to access it",
            )
                .into_response()
        }
    };

    if user_has_media {
        let media = match server_config.database.get_media(media_id.clone()).await {
            Ok(Some(media)) => media,
            Ok(None) => {
                return (
                    StatusCode::FORBIDDEN,
                    "Media does not exist or user does not have permissions to access it",
                )
                    .into_response();
            }
            Err(..) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error fetching media metadata",
                )
                    .into_response();
            }
        };
        let url = match server_config
            .bucket
            .presign_get(media_id, 86400, None)
            .await
        {
            Ok(url) => url,
            Err(..) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error creating media presigned url",
                )
                    .into_response();
            }
        };
        let media_metadata = MediaMetadataResponse {
            id: media.id,
            created_at: media.created_at,
            media_url: url,
            file_size: media.file_size,
            file_name: media.file_name,
            longitude: media.longitude,
            latitude: media.latitude,
            image_width: media.image_width,
            image_length: media.image_length,
            make: media.make,
            model: media.model,
            fnumber: media.fnumber,
            exposure_time: media.exposure_time,
            photographic_sensitivity: media.photographic_sensitivity,
            orientation: media.orientation,
        };
        (StatusCode::OK, Json(media_metadata)).into_response()
    } else {
        (
            StatusCode::FORBIDDEN,
            "Media does not exist or user does not have permissions to access it",
        )
            .into_response()
    }
}

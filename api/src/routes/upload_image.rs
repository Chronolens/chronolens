use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
};
use chrono::Utc;
use http::HeaderMap;

use crate::ServerConfig;

const ALLOWED_CONTENT_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/heic", "image/heif"];

pub async fn upload_image(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    if let Ok(Some(mut field)) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())
    {
        let digest = match field.name().map(ToString::to_string) {
            Some(digest) => digest,
            None => {
                let _ = server_config
                    .database
                    .add_log(
                        user_id,
                        database::LogLevel::Error,
                        Utc::now().timestamp_millis(),
                        "Media Upload: Could not convert the checksum into a string".to_string(),
                    )
                    .await;
                return (
                    StatusCode::BAD_REQUEST,
                    "Could not convert the checksum into a string",
                )
                    .into_response();
            }
        };

        let Some(content_type) = field.content_type().map(ToString::to_string) else {
            let _ = server_config
                .database
                .add_log(
                    user_id,
                    database::LogLevel::Error,
                    Utc::now().timestamp_millis(),
                    "Media Upload: Content type could not be decoded or does not exist".to_string(),
                )
                .await;
            return (
                StatusCode::BAD_REQUEST,
                "Content type could not be decoded or does not exist",
            )
                .into_response();
        };

        let timestamp = match headers
            .get("Timestamp")
            .and_then(|ts| ts.to_str().ok())
            .and_then(|ts| ts.parse::<i64>().ok())
        {
            Some(ts) => ts,
            None => {
                let _ = server_config
                    .database
                    .add_log(
                        user_id,
                        database::LogLevel::Error,
                        Utc::now().timestamp_millis(),
                        "Media Upload: Timestamp header missing or invalid format".to_string(),
                    )
                    .await;
                return (
                    StatusCode::BAD_REQUEST,
                    "Timestamp header missing or invalid format",
                )
                    .into_response();
            }
        };

        match server_config
            .database
            .query_media(user_id.clone(), digest.clone())
            .await
        {
            Ok(exists) => {
                if exists {
                    let _ = server_config
                        .database
                        .add_log(
                            user_id.clone(),
                            database::LogLevel::Error,
                            Utc::now().timestamp_millis(),
                            format!(
                                "Media Upload: Image with checksum {} already exists for this user",
                                digest
                            ),
                        )
                        .await;
                    return (
                        StatusCode::PRECONDITION_FAILED,
                        "Image already exists on the server",
                    )
                        .into_response();
                }
            }
            Err(..) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        if !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
            let _ = server_config
                .database
                .add_log(
                    user_id,
                    database::LogLevel::Error,
                    Utc::now().timestamp_millis(),
                    format!(
                        "Media Upload: Tried to upload the unsupported media type {}",
                        content_type.as_str()
                    ),
                )
                .await;
            return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
        }

        //Generate the media UUID
        let file_uuid = uuid::Uuid::new_v4();

        let file_name = field
            .file_name()
            .map(ToString::to_string)
            .unwrap_or(file_uuid.to_string().to_owned());

        let Ok(multipart_upload) = server_config
            .bucket
            .initiate_multipart_upload(&file_uuid.to_string(), &content_type)
            .await
        else {
            let _ = server_config
                .database
                .add_log(
                    user_id,
                    database::LogLevel::Error,
                    Utc::now().timestamp_millis(),
                    "Media Upload: Error uploading media to object storage".to_string(),
                )
                .await;
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        let mut part_number = 1;
        let mut completed_parts = vec![];
        let mut chunk_builder: Vec<u8> = vec![];
        let mut file_size: i64 = 0;

        loop {
            match field.chunk().await {
                // Case when there's a new chunk of data
                Ok(Some(data)) => {
                    file_size += data.len() as i64;
                    if chunk_builder.len() >= (5 * 1024 * 1024) {
                        let Ok(upload_response) = server_config
                            .bucket
                            .put_multipart_chunk(
                                chunk_builder.clone(),
                                &file_uuid.to_string(),
                                part_number,
                                &multipart_upload.upload_id,
                                &content_type,
                            )
                            .await
                        else {
                            let _ = server_config
                                .database
                                .add_log(
                                    user_id,
                                    database::LogLevel::Error,
                                    Utc::now().timestamp_millis(),
                                    "Media Upload: Error uploading media to object storage"
                                        .to_string(),
                                )
                                .await;
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        };
                        // Store the ETag of this part
                        completed_parts.push(upload_response);
                        part_number += 1;
                        chunk_builder.clear();
                        chunk_builder.append(&mut data.to_vec());
                    } else {
                        chunk_builder.append(&mut data.to_vec());
                    }
                }

                // Case when there are no more chunks (end of file/stream)
                Ok(None) => {
                    let Ok(upload_response) = server_config
                        .bucket
                        .put_multipart_chunk(
                            chunk_builder,
                            &file_uuid.to_string(),
                            part_number,
                            &multipart_upload.upload_id,
                            &content_type,
                        )
                        .await
                    else {
                        let _ = server_config
                            .database
                            .add_log(
                                user_id,
                                database::LogLevel::Error,
                                Utc::now().timestamp_millis(),
                                "Media Upload: Error uploading media to object storage".to_string(),
                            )
                            .await;
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    };
                    completed_parts.push(upload_response);
                    // Step 3: Complete multipart upload
                    match server_config
                        .bucket
                        .complete_multipart_upload(
                            &file_uuid.to_string(),
                            &multipart_upload.upload_id,
                            completed_parts,
                        )
                        .await
                    {
                        Err(..) => {
                            let _ = server_config
                                .database
                                .add_log(
                                    user_id,
                                    database::LogLevel::Error,
                                    Utc::now().timestamp_millis(),
                                    "Media Upload: Error uploading media to object storage"
                                        .to_string(),
                                )
                                .await;
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                        Ok(..) => {
                            let Ok(_) = server_config
                                .database
                                .add_media(
                                    user_id.clone(),
                                    file_uuid.to_string(),
                                    digest,
                                    timestamp,
                                    file_size,
                                    file_name.to_owned(),
                                )
                                .await
                            else {
                                //If adding to the DB fails, remove the file from the object storage
                                let _ = server_config
                                    .database
                                    .add_log(
                                        user_id,
                                        database::LogLevel::Error,
                                        Utc::now().timestamp_millis(),
                                        "Media Upload: Error uploading media to object storage"
                                            .to_string(),
                                    )
                                    .await;
                                server_config
                                    .bucket
                                    .delete_object(file_uuid.to_string())
                                    .await
                                    .unwrap();
                                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                            };
                        }
                    };

                    // Step 4: publish preview generation request
                    let file_uuid_bytes = Bytes::from(String::from(file_uuid));
                    let _ = match server_config
                        .nats_jetstream
                        .publish("previews", file_uuid_bytes.clone())
                        .await
                    {
                        Ok(publish_ack) => publish_ack,
                        Err(..) => {
                            let _ = server_config
                                .database
                                .add_log(
                                    user_id,
                                    database::LogLevel::Error,
                                    Utc::now().timestamp_millis(),
                                    "Media Upload: Error publishing picture to NATS".to_string(),
                                )
                                .await;
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                    };

                    // Step 5: publish ml embeddings generation request
                    let _ = match server_config
                        .nats_jetstream
                        .publish("image-process", file_uuid_bytes.clone())
                        .await
                    {
                        Ok(publish_ack) => publish_ack,
                        Err(..) => {
                            let _ = server_config
                                .database
                                .add_log(
                                    user_id,
                                    database::LogLevel::Error,
                                    Utc::now().timestamp_millis(),
                                    "Media Upload: Error publishing picture to NATS".to_string(),
                                )
                                .await;
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                    };

                    // Step 6: publish metadata request
                    let _ = match server_config
                        .nats_jetstream
                        .publish("metadata", file_uuid_bytes)
                        .await
                    {
                        Ok(publish_ack) => publish_ack,
                        Err(..) => {
                            let _ = server_config
                                .database
                                .add_log(
                                    user_id,
                                    database::LogLevel::Error,
                                    Utc::now().timestamp_millis(),
                                    "Media Upload: Error publishing picture to NATS".to_string(),
                                )
                                .await;
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                    };

                    let _ = server_config
                        .database
                        .add_log(
                            user_id,
                            database::LogLevel::Info,
                            Utc::now().timestamp_millis(),
                            format!(
                                "Media Upload: {} uploaded successfully with id {}",
                                file_name, file_uuid
                            ),
                        )
                        .await;

                    break;
                }
                // Case when an error occurs
                Err(_) => {
                    let _ = server_config
                        .database
                        .add_log(
                            user_id,
                            database::LogLevel::Error,
                            Utc::now().timestamp_millis(),
                            "Media Upload: Error receiving file from the client".to_string(),
                        )
                        .await;
                    return StatusCode::BAD_REQUEST.into_response();
                }
            }
        }
        return (StatusCode::OK, file_uuid.to_string()).into_response();
    }
    let _ = server_config
        .database
        .add_log(
            user_id,
            database::LogLevel::Error,
            Utc::now().timestamp_millis(),
            "Media Upload: No field in multipart upload".to_string(),
        )
        .await;
    (StatusCode::BAD_REQUEST).into_response()
}

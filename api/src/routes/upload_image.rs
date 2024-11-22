use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
};
use http::HeaderMap;

use crate::ServerConfig;

const _ALLOWED_CONTENT_TYPES: [&str; 3] = ["image/png", "image/heic", "image/jpeg"];

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
        let digest = field
            .name()
            .map(ToString::to_string)
            .unwrap_or("name".to_owned());

        let Some(content_type) = field.content_type().map(ToString::to_string) else {
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
                return (
                    StatusCode::BAD_REQUEST,
                    "Timestamp header missing or invalid format",
                )
                    .into_response()
            }
        };

        let image_exists = match server_config
            .database
            .query_media(user_id.clone(), digest.clone())
            .await
        {
            Ok(exists) => exists,
            Err(..) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        if image_exists {
            return (
                StatusCode::PRECONDITION_FAILED,
                "Image already exists on the server",
            )
                .into_response();
        }

        // FIX: UNCOMMENT THIS CONDITION!

        //if ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
        //    return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
        //}

        //Generate the media UUID
        let file_uuid = uuid::Uuid::new_v4();

        let Ok(multipart_upload) = server_config
            .bucket
            .initiate_multipart_upload(&file_uuid.to_string(), &content_type)
            .await
        else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        let mut part_number = 1;
        let mut completed_parts = vec![];
        let mut chunk_builder: Vec<u8> = vec![];

        loop {
            match field.chunk().await {
                // Case when there's a new chunk of data
                Ok(Some(data)) => {
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
                    println!("Finished receiving file");
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
                                )
                                .await
                            else {
                                //If adding to the DB fails, remove the file from the object storage
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
                        Err(..) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    };

                    // Step 5: publish ml embeddings generation request
                    let _ = match server_config
                        .nats_jetstream
                        .publish("machine-learning", file_uuid_bytes)
                        .await
                    {
                        Ok(publish_ack) => publish_ack,
                        Err(..) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    };
                    break;
                }
                // Case when an error occurs
                Err(err) => {
                    println!("Error: {}", err);
                    return StatusCode::BAD_REQUEST.into_response();
                }
            }
        }
        return (StatusCode::OK, file_uuid.to_string()).into_response();
    }
    (StatusCode::BAD_REQUEST).into_response()
}

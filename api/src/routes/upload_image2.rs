
use crate::ServerConfig;
use axum::extract::Request;
use axum::response::Response;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension};
use base64::engine::general_purpose;
use base64::Engine;
use futures_util::StreamExt;
use http::header::CONTENT_TYPE;
use http::HeaderMap;

pub async fn upload_image2(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
    headers: HeaderMap,
    request: Request,
) -> impl IntoResponse {
    let digest = match get_content_digest(&headers) {
        Ok(digest) => digest,
        Err(response) => return response,
    };

    let content_type = match headers.get(CONTENT_TYPE).and_then(|ct| ct.to_str().ok()) {
        Some(ct) => ct,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Content-Type header could not be decoded or does not exist",
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
            StatusCode::BAD_REQUEST,
            "Image already exists on the server",
        )
            .into_response();
    }

    //Generate the media UUID
    let file_uuid = uuid::Uuid::new_v4();

    let Ok(multipart_upload) = server_config
        .bucket
        .initiate_multipart_upload(&file_uuid.to_string(), content_type)
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut part_number = 1;
    let mut completed_parts = vec![];
    let mut chunk_builder: Vec<u8> = vec![];

    // Process each chunk as soon as it arrives without accumulating chunks in memory
    let mut stream = request.into_body().into_data_stream();
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Each chunk is processed and discarded immediately after processing
                if chunk_builder.len() >= (5 * 1024 * 1024) {
                    let Ok(upload_response) = server_config
                        .bucket
                        .put_multipart_chunk(
                            chunk_builder.clone(),
                            &file_uuid.to_string(),
                            part_number,
                            &multipart_upload.upload_id,
                            content_type,
                        )
                        .await
                    else {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    };
                    // Store the ETag of this part
                    completed_parts.push(upload_response);
                    part_number += 1;
                    chunk_builder.clear();
                    chunk_builder.append(&mut chunk.to_vec());
                } else {
                    chunk_builder.append(&mut chunk.to_vec());
                }
            }
            Err(..) => {
                // Handle the error if reading the chunk fails
                return StatusCode::BAD_REQUEST.into_response();
            }
        }
    }

    let Ok(upload_response) = server_config
        .bucket
        .put_multipart_chunk(
            chunk_builder,
            &file_uuid.to_string(),
            part_number,
            &multipart_upload.upload_id,
            content_type,
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
                .add_media(user_id, file_uuid.to_string(), digest)
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

    StatusCode::OK.into_response()
}

fn get_content_digest(headers: &HeaderMap) -> Result<Vec<u8>, Response> {
    let checksum_header = match headers.get("Content-Digest") {
        Some(header) => header.to_str().unwrap(),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                "No checksum for body found in headers (Content-Digest)",
            )
                .into_response())
        }
    };

    let encoded_digest = match checksum_header
        .strip_prefix("sha-256=:")
        .and_then(|checksum| checksum.strip_suffix(":"))
    {
        Some(checksum) => checksum.to_string(),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid checksum format, please use 'sha-256=:base64_hash_here:'",
            )
                .into_response())
        }
    };

    match general_purpose::STANDARD.decode(encoded_digest) {
        Ok(digest) => Ok(digest),
        Err(_) => Err((
            StatusCode::BAD_REQUEST,
            "No checksum for body found in headers (Content-Digest)",
        )
            .into_response()),
    }
}
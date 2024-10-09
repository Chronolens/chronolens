use axum::{body::Body, extract::State, http::StatusCode, response::IntoResponse};
use futures_util::StreamExt;

use crate::ServerConfig;

pub async fn upload_image2(
    State(server_config): State<ServerConfig>,
    body: Body,
) -> impl IntoResponse {
    println!("Receiving file");

    //Get the first chunk
    let mut stream = body.into_data_stream();
    let first_chunk = match stream.next().await {
        Some(Ok(chunk)) => chunk,
        Some(Err(..)) => return StatusCode::BAD_REQUEST.into_response(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let (checksum, first_file_chunk) = first_chunk.split_at(32);

    let image_exists = match server_config.database.query_image(checksum.to_vec()).await {
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
        .initiate_multipart_upload(&file_uuid.to_string(), "image/png")
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut part_number = 1;
    let mut completed_parts = vec![];
    let mut chunk_builder: Vec<u8> = vec![];
    chunk_builder.append(&mut first_file_chunk.to_vec());

    // Process each chunk as soon as it arrives without accumulating chunks in memory
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
                            "image/png",
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
            Err(_) => {
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
            "image/png",
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
        Err(err) => {
            println!("Error: {}", err);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        Ok(..) => println!("Multipart upload complete"),
    };

    StatusCode::OK.into_response()
}

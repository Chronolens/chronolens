use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::{models::api_models::UploadImageResponse, ServerConfig};

//const ALLOWED_CONTENT_TYPES: [&str; 3] = ["image/png", "image/heic", "image/jpeg"];

pub async fn upload_image(
    State(server_config): State<ServerConfig>,
    mut multipart: Multipart,
) -> Response {
    println!("Receiving upload image request");

    while let Ok(Some(mut field)) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())
    {
        let _name = match field.name() {
            Some(name) => name.to_owned(),
            None => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response(),
        };
        let file_name = match field.file_name() {
            Some(file_name) => file_name.to_owned(),
            None => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response(),
        };
        let content_type = match field.content_type() {
            Some(content_type) => content_type.to_owned(),
            None => {
                return StatusCode::BAD_REQUEST.into_response();
            }
        };

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
                    println!("Finished receiving {}", file_name);
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
                        Err(err) => {
                            println!("Error: {}", err);
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                        Ok(..) => println!("Multipart upload complete for {}", file_name),
                    };

                    // TODO: Add image to db
                    break;
                }

                // Case when an error occurs
                Err(err) => {
                    println!("Error: {}", err);
                    return StatusCode::BAD_REQUEST.into_response();
                }
            }
        }
    }
    (StatusCode::OK, Json(UploadImageResponse { size: 64 })).into_response()
}

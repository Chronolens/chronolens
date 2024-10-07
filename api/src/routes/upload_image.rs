use std::time::Duration;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use tokio::time::Instant;

use crate::{models::api_models::UploadImageResponse, ServerConfig};

pub async fn upload_image(
    State(_server_config): State<ServerConfig>,
    mut multipart: Multipart,
) -> Response {
    let message = multipart.next_field().await.unwrap();
    let mut field = match message {
        Some(field) => field,
        None => return (StatusCode::BAD_REQUEST).into_response(),
    };

    let file_name = field
        .file_name()
        .map(ToString::to_string)
        .unwrap_or("file_name".to_owned());

    let mut last_time = Instant::now(); // Time checkpoint for throughput measurements
    let mut size = 0;
    let mut bytes_in_last_second = 0; // Tracks bytes transferred in the last second

    loop {
        match field.chunk().await {
            // Case when there's a new chunk of data
            Ok(Some(data)) => {
                let chunk_size = data.len() as u64;
                size += chunk_size;
                bytes_in_last_second += chunk_size;
                println!("Chunk size: {}", chunk_size);

                if last_time.elapsed() >= Duration::from_secs(1) {
                    let elapsed = last_time.elapsed().as_secs_f64();
                    let throughput = bytes_in_last_second as f64 / elapsed;
                    println!("Throughput: {:.2} MB/sec", throughput / (1024.0 * 1024.0));

                    last_time = Instant::now();
                    bytes_in_last_second = 0;
                }
                // Add chunk to S3
            }

            // Case when there are no more chunks (end of file/stream)
            Ok(None) => {
                println!("Finished receiving {}", file_name);

                // Add image to database
                break;
            }

            // Case when an error occurs
            Err(err) => {
                println!("Error: {}", err);
                return StatusCode::BAD_REQUEST.into_response();
            }
        }
    }


    (StatusCode::OK, Json(UploadImageResponse { size })).into_response()
}

use std::{fs::File, io::Write, time::Duration};

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

    let mut file = File::create(&file_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();

    let mut last_time = Instant::now(); // Time checkpoint for throughput measurements
    let mut size = 0;
    let mut bytes_in_last_second = 0; // Tracks bytes transferred in the last second

    while let Ok(Some(data)) = field.chunk().await.map_err(|err| {
        println!("Error: {}", err);
        StatusCode::BAD_REQUEST.into_response()
    }) {
        let chunk_size = data.len() as u64;
        size += chunk_size;
        bytes_in_last_second += chunk_size;
        println!("Chunk size: {}",chunk_size);

        // Check if at least 1 second has passed
        if last_time.elapsed() >= Duration::from_secs(1) {
            let elapsed = last_time.elapsed().as_secs_f64();
            let throughput = bytes_in_last_second as f64 / elapsed; // Throughput for the last second
            println!(
                "Throughput: {:.2} MB/sec", throughput / (1024.0 *1024.0)
            );

            // Reset for the next second
            last_time = Instant::now();
            bytes_in_last_second = 0;
        }

        file.write_all(&data)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
            .unwrap();
        file.flush().unwrap();
    }

    println!("Finished receiving {}", file_name);

    (StatusCode::OK, Json(UploadImageResponse { size })).into_response()
}

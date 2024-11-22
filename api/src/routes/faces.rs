use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use futures_util::future::join_all;
use http::StatusCode;

use crate::{
    models::api_models::{ClusterResponse, FaceResponse, GetFacesResponse},
    ServerConfig,
};

pub async fn faces(
    State(server_config): State<ServerConfig>,
    Extension(user_id): Extension<String>,
) -> Response {
    match server_config.database.get_faces(user_id).await {
        Ok((faces, clusters)) => {
            let sc1 = &server_config.clone();
            let sc2 = &server_config.clone();
            let face_futures: Vec<_> = faces
                .into_iter()
                .map(|face| async move {
                    let photo_url = sc1
                        .bucket
                        .presign_get(&face.photo_id, 86400, None)
                        .await
                        .unwrap();
                    FaceResponse {
                        face_id: face.face_id,
                        name: face.name,
                        photo_url: photo_url.to_string(),
                        bbox: face.bbox,
                    }
                })
                .collect();

            let cluster_futures: Vec<_> = clusters
                .into_iter()
                .map(|cluster| async move {
                    let photo_url = sc2
                        .bucket
                        .presign_get(&cluster.photo_id, 86400, None)
                        .await
                        .unwrap();
                    ClusterResponse {
                        cluster_id: cluster.cluster_id,
                        photo_url: photo_url.to_string(),
                        bbox: cluster.bbox,
                    }
                })
                .collect();
            let face_responses = join_all(face_futures).await;
            let cluster_responses = join_all(cluster_futures).await;
            (
                StatusCode::OK,
                Json(GetFacesResponse {
                    faces: face_responses,
                    clusters: cluster_responses,
                }),
            )
                .into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

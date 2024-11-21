pub mod schema;

use migration::{Migrator, MigratorTrait};
use schema::{
    cluster, face, log,
    media::{self, ActiveModel},
    media_face, user,
};
use sea_orm::{
    entity::*, query::*, sqlx::types::chrono::Utc, ColumnTrait, ConnectOptions, Database,
    DatabaseConnection, DbErr, EntityTrait, FromQueryResult, QueryFilter,
};
use serde::{Deserialize, Serialize};
use std::string::ToString;

#[derive(Deserialize, Debug)]
struct DbEnvs {
    #[serde(alias = "DATABASE_USERNAME")]
    database_username: String,
    #[serde(alias = "DATABASE_PASSWORD")]
    database_password: String,
    #[serde(alias = "DATABASE_HOST")]
    database_host: String,
    #[serde(alias = "DATABASE_PORT")]
    #[serde(default = "default_port")]
    database_port: u16,
    #[serde(alias = "DATABASE_NAME")]
    database_name: String,
}
fn default_port() -> u16 {
    5432
}

#[derive(Clone)]
pub struct DbManager {
    pub connection: DatabaseConnection,
}

impl DbManager {
    pub async fn new() -> Result<Self, DbErr> {
        let db_config = envy::from_env::<DbEnvs>().unwrap();
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            db_config.database_username,
            db_config.database_password,
            db_config.database_host,
            db_config.database_port,
            db_config.database_name
        );
        let mut opt = ConnectOptions::new(&connection_string);
        opt.max_connections(100).min_connections(5);
        let connection: DatabaseConnection = Database::connect(opt).await?;
        Migrator::up(&connection, None).await?;
        Ok(DbManager { connection })
    }

    pub async fn query_media(&self, user_id: String, checksum: String) -> Result<bool, &str> {
        match media::Entity::find()
            .filter(media::Column::Hash.eq(checksum))
            .filter(media::Column::UserId.eq(user_id))
            .one(&self.connection)
            .await
        {
            Ok(Some(..)) => Ok(true),
            Ok(None) => Ok(false),
            Err(err) => {
                println!("Err: {}", err);
                Err("Failed to query checksum")
            }
        }
    }

    pub async fn add_media(
        &self,
        user_id: String,
        media_id: String,
        checksum: String,
        timestamp: i64,
    ) -> Result<InsertResult<ActiveModel>, DbErr> {
        let media_to_insert = media::ActiveModel {
            id: Set(media_id),
            user_id: Set(user_id),
            preview_id: Set(None),
            hash: Set(checksum),
            created_at: Set(timestamp),
            last_modified_at: Set(Utc::now().timestamp_millis()),
            deleted: Set(false),
        };

        media::Entity::insert(media_to_insert)
            .exec(&self.connection)
            .await
    }
    pub async fn get_media(&self, media_id: String) -> Result<Option<media::Model>, DbErr> {
        media::Entity::find_by_id(&media_id)
            .one(&self.connection)
            .await
    }

    async fn _delete_media(&self, media_id: i32, user_id: i32) -> Result<(), &'static str> {
        // Find the photo to be deleted
        let media = media::Entity::find()
            .filter(media::Column::Id.eq(media_id))
            .filter(media::Column::UserId.eq(user_id))
            .one(&self.connection)
            .await;

        match media {
            Ok(Some(media)) => {
                let mut media_model = media.into_active_model();
                media_model.deleted = Set(true);
                media_model.last_modified_at = Set(Utc::now().timestamp_millis());
                // Save changes to the database
                match media_model.update(&self.connection).await {
                    Ok(_) => Ok(()), // Return success if updated
                    Err(..) => Err("Failed to delete media"),
                }
            }
            Ok(None) => Err("Failed to delete media"),
            Err(..) => Err("Error "),
        }
    }

    pub async fn get_user(&self, username: String) -> Result<user::Model, GetUserError> {
        match user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.connection)
            .await
        {
            Ok(Some(user)) => Ok(user),
            Ok(None) => Err(GetUserError::NotFound),
            Err(..) => Err(GetUserError::InternalError),
        }
    }
    pub async fn update_media_preview(
        &self,
        media_id: String,
        preview_id: String,
    ) -> Result<(), String> {
        let Ok(media) = media::Entity::find_by_id(&media_id)
            .one(&self.connection)
            .await
        else {
            return Err(format!(
                "Database error while fetching media: {}",
                media_id.clone()
            ));
        };
        let Some(media) = media else {
            return Err(format!(
                "Could not find media: {} in the database",
                media_id.clone()
            ));
        };
        let mut media: media::ActiveModel = media.into();
        // WARN: should i rewrite this no matter what?
        media.preview_id = Set(Some(preview_id));
        match media.update(&self.connection).await {
            Ok(_) => Ok(()),
            Err(_) => Err(format!(
                "Could not update media preview id for: {}",
                media_id.clone()
            )),
        }
    }

    pub async fn sync_full(&self, user_id: String) -> Result<Vec<RemoteMediaAdded>, &str> {
        match media::Entity::find()
            .select_only()
            .select_column(media::Column::Id)
            .select_column(media::Column::CreatedAt)
            .select_column(media::Column::Hash)
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::Deleted.eq(false))
            .into_model::<RemoteMediaAdded>()
            .all(&self.connection)
            .await
        {
            Ok(user) => Ok(user),
            Err(err) => {
                println!("Err: {}", err);
                Err("Failed to get media")
            }
        }
    }

    pub async fn sync_partial(
        &self,
        user_id: String,
        since: i64,
    ) -> Result<(Vec<RemoteMediaAdded>, Vec<RemoteMediaDeleted>), &str> {
        // Query for added media
        let changed_media = media::Entity::find()
            .select_only()
            .select_column(media::Column::Id)
            .select_column(media::Column::CreatedAt)
            .select_column(media::Column::Hash)
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::LastModifiedAt.gt(since));

        let added_media = changed_media
            .clone()
            .filter(media::Column::Deleted.eq(false))
            .into_model::<RemoteMediaAdded>()
            .all(&self.connection)
            .await;

        // Query for deleted media
        let deleted_media = changed_media
            .clone()
            .filter(media::Column::Deleted.eq(true)) // Deleted
            .into_model::<RemoteMediaDeleted>()
            .all(&self.connection)
            .await;

        match (added_media, deleted_media) {
            (Ok(added), Ok(deleted)) => Ok((added, deleted)), // Return two vectors: added and deleted media
            (Err(_), _) | (_, Err(_)) => Err("Failed to get media changes"),
        }
    }

    pub async fn user_has_media(&self, user_id: String, media_id: &String) -> Result<bool, &str> {
        match media::Entity::find()
            .select_only()
            .select_column(media::Column::Id)
            .filter(media::Column::Id.eq(media_id))
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::Deleted.eq(false))
            .into_tuple::<String>()
            .one(&self.connection)
            .await
        {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err("Failed to get media"),
        }
    }

    pub async fn get_previews(
        &self,
        user_id: String,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<(String, String)>, GetPreviewError> {
        let offset = (page - 1) * page_size;

        match media::Entity::find()
            .order_by_desc(media::Column::CreatedAt)
            .select_only()
            .select_column(media::Column::Id)
            .select_column(media::Column::PreviewId)
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::Deleted.eq(false))
            .offset(offset)
            .limit(page_size)
            .into_tuple::<(String, String)>()
            .all(&self.connection)
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => Err(GetPreviewError::InternalError),
        }
    }

    pub async fn get_preview_from_user(
        &self,
        user_id: String,
        media_id: &String,
    ) -> Result<String, GetPreviewError> {
        match media::Entity::find()
            .select_only()
            .select_column(media::Column::PreviewId)
            .filter(media::Column::Id.eq(media_id))
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::Deleted.eq(false))
            .into_tuple::<String>()
            .one(&self.connection)
            .await
        {
            Ok(Some(preview_id)) => Ok(preview_id),
            Ok(None) => Err(GetPreviewError::NotFound),
            Err(_) => Err(GetPreviewError::InternalError),
        }
    }

    pub async fn get_logs(
        &self,
        user_id: String,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<LogEntry>, GetLogError> {
        let offset = (page - 1) * page_size;

        match log::Entity::find()
            .order_by_desc(log::Column::Date)
            .filter(log::Column::UserId.eq(user_id))
            .offset(offset)
            .limit(page_size)
            .all(&self.connection)
            .await
        {
            Ok(log_models) => {
                let log_entries = log_models
                    .into_iter()
                    .map(|model| LogEntry {
                        id: model.id,
                        level: model.level,
                        date: model.date,
                        message: model.message,
                    })
                    .collect();
                Ok(log_entries)
            }
            Err(_) => Err(GetLogError::InternalError),
        }
    }

    pub async fn add_log(
        &self,
        user_id: String,
        level: LogLevel,
        date: i64,
        message: String,
    ) -> Result<(), AddLogError> {
        let new_log = log::ActiveModel {
            user_id: Set(user_id),
            level: Set(level.to_string()),
            date: Set(date),
            message: Set(message),
            ..Default::default()
        };

        match new_log.insert(&self.connection).await {
            Ok(_) => Ok(()),
            Err(_) => Err(AddLogError::InternalError),
        }
    }

    pub async fn get_faces(&self, user_id: String) -> Result<(Vec<Face>, Vec<Cluster>), DbErr> {
        // Query 1: Get all clusters *without* faces
        let clusters_without_faces: Vec<(cluster::Model, Option<face::Model>)> =
            cluster::Entity::find()
                .filter(cluster::Column::UserId.eq(user_id.clone()))
                .filter(face::Column::Id.is_null())
                .find_also_related(face::Entity)
                .all(&self.connection)
                .await?;

        // Query 2: Get distinct clusters *with* faces based on FaceId
        let clusters_with_faces: Vec<(cluster::Model, Option<face::Model>)> =
            cluster::Entity::find()
                .filter(cluster::Column::UserId.eq(user_id.clone()))
                .filter(face::Column::Id.is_not_null())
                .find_also_related(face::Entity)
                .distinct_on([cluster::Column::FaceId])
                .all(&self.connection)
                .await?;

        // Combine results
        let faces_with_clusters: Vec<(cluster::Model, Option<face::Model>)> =
            clusters_without_faces
                .into_iter()
                .chain(clusters_with_faces.into_iter())
                .collect();

        let mut faces: Vec<Face> = vec![];
        let mut clusters: Vec<Cluster> = vec![];
        for (cluster, face_opt) in faces_with_clusters {
            if let Some(face) = face_opt {
                let (photo_id, bbox) = if let Some(featured_photo_id) = &face.featured_photo_id {
                    media_face::Entity::find()
                        .select_only()
                        .select_column(media_face::Column::MediaId)
                        .select_column(media_face::Column::FaceBoundingBox)
                        .filter(media_face::Column::MediaId.eq(featured_photo_id.clone()))
                        .into_tuple()
                        .one(&self.connection)
                        .await?
                        .unwrap()
                } else {
                    media_face::Entity::find()
                        .select_only()
                        .select_column(media_face::Column::MediaId)
                        .select_column(media_face::Column::FaceBoundingBox)
                        .filter(media_face::Column::ClusterId.eq(cluster.id))
                        .into_tuple()
                        .one(&self.connection)
                        .await?
                        .unwrap()
                };

                faces.push(Face {
                    face_id: face.id,
                    name: face.name.clone(),
                    photo_id,
                    bbox,
                });
            } else {
                let (photo_id, bbox) = media_face::Entity::find()
                    .select_only()
                    .select_column(media_face::Column::MediaId)
                    .select_column(media_face::Column::FaceBoundingBox)
                    .filter(media_face::Column::ClusterId.eq(cluster.id))
                    .into_tuple()
                    .one(&self.connection)
                    .await?
                    .unwrap();
                clusters.push(Cluster {
                    cluster_id: cluster.id,
                    photo_id,
                    bbox,
                });
            }
        }
        Ok((faces, clusters))
    }

    pub async fn get_cluster_previews(
        &self,
        user_id: String,
        cluster_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<(String, String)>, GetPreviewError> {
        let offset = (page - 1) * page_size;

        match media_face::Entity::find()
            .filter(media_face::Column::ClusterId.eq(cluster_id))
            .join(JoinType::LeftJoin, media_face::Relation::Media.def())
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::Deleted.eq(false))
            .order_by_desc(media::Column::CreatedAt)
            .select_only()
            .column_as(media::Column::Id, "media_id")
            .column_as(media::Column::PreviewId, "preview_id")
            .offset(offset)
            .limit(page_size)
            .into_tuple::<(String, String)>()
            .all(&self.connection)
            .await
        {
            Ok(results) => Ok(results),
            Err(_) => Err(GetPreviewError::InternalError),
        }
    }

    pub async fn get_face_previews(
        &self,
        user_id: String,
        face_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<(String, String)>, GetPreviewError> {
        let offset = (page - 1) * page_size;

        match media_face::Entity::find()
            .join(JoinType::InnerJoin, media_face::Relation::Cluster.def())
            .filter(cluster::Column::FaceId.eq(face_id))
            .join(JoinType::LeftJoin, media_face::Relation::Media.def())
            .filter(media::Column::UserId.eq(user_id))
            .filter(media::Column::Deleted.eq(false))
            .order_by_desc(media::Column::CreatedAt)
            .select_only()
            .column_as(media::Column::Id, "media_id")
            .column_as(media::Column::PreviewId, "preview_id")
            .offset(offset)
            .limit(page_size)
            .into_tuple::<(String, String)>()
            .all(&self.connection)
            .await
        {
            Ok(results) => Ok(results),
            Err(_) => Err(GetPreviewError::InternalError),
        }
    }
}

pub enum GetPreviewError {
    NotFound,
    InternalError,
}

pub enum GetLogError {
    InternalError,
    NotFound,
}

pub enum AddLogError {
    InternalError,
}

pub enum GetUserError {
    NotFound,
    InternalError,
}

#[derive(strum_macros::Display, Debug)]
pub enum LogLevel {
    Info,
    Error,
}

#[derive(Serialize)]
pub struct LogEntry {
    pub id: i32,
    pub level: String,
    pub date: i64,
    pub message: String,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct LogResponse {
    pub logs: Vec<LogEntry>,
}

#[derive(Deserialize, Serialize, Debug, Clone, FromQueryResult)]
pub struct RemoteMediaAdded {
    pub id: String,
    pub created_at: i64,
    pub hash: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, FromQueryResult)]
#[serde(transparent)]
pub struct RemoteMediaDeleted {
    pub id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, FromQueryResult)]
pub struct Face {
    pub face_id: i32,
    pub name: String,
    pub photo_id: String,
    pub bbox: Vec<i32>,
}

#[derive(Deserialize, Serialize, Debug, Clone, FromQueryResult)]
pub struct Cluster {
    pub cluster_id: i32,
    pub photo_id: String,
    pub bbox: Vec<i32>,
}

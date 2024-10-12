pub mod schema;

use migration::{Migrator, MigratorTrait};
use schema::{
    media::{self, ActiveModel},
    user,
};
use sea_orm::{
    entity::*, query::*, sqlx::types::chrono::Utc, ColumnTrait, ConnectOptions, Database,
    DatabaseConnection, DbErr, EntityTrait, FromQueryResult, QueryFilter,
};
use serde::Deserialize;

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

    pub async fn get_user(&self, username: String) -> Result<user::Model, &str> {
        match user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.connection)
            .await
        {
            Ok(user) => Ok(user.expect("Username not found")),
            Err(err) => {
                println!("Err: {}", err);
                Err("Failed to get user")
            }
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
            (Err(_), _) | (_, Err(_)) => {
                Err("Failed to get media changes")
            }
        }
    }

    pub async fn user_has_media(&self, user_id: String,media_id: &String) -> Result<bool, &str> {
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
            Err(_) => {
                Err("Failed to get media")
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone, FromQueryResult)]
pub struct RemoteMediaAdded {
    pub id: String,
    pub created_at: i64,
    pub hash: String,
}

#[derive(Deserialize, Debug, Clone, FromQueryResult)]
pub struct RemoteMediaDeleted {
    pub id: String,
}

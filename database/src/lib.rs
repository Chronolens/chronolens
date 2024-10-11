pub mod schema;

use migration::{Migrator, MigratorTrait};
use schema::{
    media::{self, ActiveModel},
    user,
};
use sea_orm::{
    entity::*, query::*, sqlx::types::chrono::Utc, ColumnTrait, ConnectOptions, Database,
    DatabaseConnection, DbErr, EntityTrait, QueryFilter,
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

    pub async fn query_media(&self, user_id: String, checksum: Vec<u8>) -> Result<bool, &str> {
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
        checksum: Vec<u8>,
        timestamp: i64,
    ) -> Result<InsertResult<ActiveModel>, DbErr> {
        let media_to_insert = media::ActiveModel {
            id: Set(media_id),
            user_id: Set(user_id),
            preview_id: Set(None),
            hash: Set(checksum),
            created_at: Set(timestamp),
            uploaded_at: Set(Utc::now().timestamp_millis()),
        };

        media::Entity::insert(media_to_insert)
            .exec(&self.connection)
            .await
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
    ) -> Result<(), &str> {
        let Ok(media) = media::Entity::find_by_id(media_id)
            .one(&self.connection)
            .await
        else {
            return Err(format!("Database error while fetching media: {media_id}").as_str());
        };
        let Some(media) = media else {
            return Err(format!("Could not find media: {media_id} in the database").as_str());
        };
        let mut media: media::ActiveModel = media.into();
        // should i rewrite this no matter what?
        media.preview_id = Set(preview_id);

        return Ok(());
    }
}

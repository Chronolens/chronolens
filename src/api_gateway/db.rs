use chronolens::schema::user;
use sea_orm::{
    ColumnTrait, ConnectOptions, Database, DatabaseConnection, DbErr, EntityTrait, FromQueryResult,
    QueryFilter, QuerySelect,
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

pub struct DbAccess {
    pub connection: DatabaseConnection,
}

impl DbAccess {
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
        Ok(DbAccess { connection })
    }

    pub async fn get_user_password(&self, username: String) -> Result<UserPassword, &str> {
        match user::Entity::find()
            .select_only()
            .column(user::Column::Password)
            .filter(user::Column::Username.eq(username))
            .into_model::<UserPassword>()
            .one(&self.connection)
            .await
        {
            Ok(password_hash) => Ok(password_hash.expect("Username or password not found")),
            Err(err) => {
                println!("Err: {}", err);
                Err("Failed to get user password")
            }
        }
    }
}

#[derive(Debug, FromQueryResult)]
pub struct UserPassword {
    pub password: String,
}

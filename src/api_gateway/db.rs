use chronolens::schema::user;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{ConnectionError, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
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
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl DbAccess {
    pub fn new() -> Result<Self, ConnectionError> {
        let db_config = envy::from_env::<DbEnvs>().unwrap();
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            db_config.database_username,
            db_config.database_password,
            db_config.database_host,
            db_config.database_port,
            db_config.database_name
        );
        let manager = ConnectionManager::<PgConnection>::new(&connection_string);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");
        Ok(DbAccess { pool })
    }
}

pub fn get_user_password(connection: &mut PgConnection, username: String) -> Result<String, &str> {
    #[derive(Insertable)]
    #[diesel(table_name = user)]
    struct LoginRequest {
        username: String,
        password: String,
    }
    match user::dsl::user
        .filter(user::columns::username.eq(&username))
        .select(user::columns::password)
        .first::<String>(connection)
    {
        Ok(password_hash) => Ok(password_hash),
        Err(..) => Err("Failed to get user password"),
    }
}

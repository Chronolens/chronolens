pub use sea_orm_migration::prelude::*;

mod m002_user;
mod m003_media;
mod m004_face_data;
mod m005_log;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m002_user::Migration),
            Box::new(m003_media::Migration),
            Box::new(m004_face_data::Migration),
            Box::new(m005_log::Migration),
        ]
    }
}

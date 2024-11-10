pub use sea_orm_migration::prelude::*;

mod m001_setup;
mod m002_user;
mod m003_media;
mod m004_face;
mod m005_media_face;
mod m006_cluster;
mod m007_log;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m001_setup::Migration),
            Box::new(m002_user::Migration),
            Box::new(m003_media::Migration),
            Box::new(m004_face::Migration),
            Box::new(m005_media_face::Migration),
            Box::new(m006_cluster::Migration),
            Box::new(m007_log::Migration),
        ]
    }
}

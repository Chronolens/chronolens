pub use sea_orm_migration::prelude::*;

mod m002_user;
mod m003_media;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m002_user::Migration),
            Box::new(m003_media::Migration),
        ]
    }
}

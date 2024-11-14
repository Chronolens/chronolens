use crate::{m002_user::User, m003_media::Media};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Face::Table)
                    .if_not_exists()
                    .col(integer(Face::Id).primary_key().auto_increment())
                    .col(string(Face::Name))
                    .col(string_null(Face::FeaturedPhotoId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("featured_photo_id")
                            .from(Face::Table, Face::FeaturedPhotoId)
                            .to(Media::Table, Media::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Face::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Face {
    Table,
    Id,
    Name,
    FeaturedPhotoId,
}

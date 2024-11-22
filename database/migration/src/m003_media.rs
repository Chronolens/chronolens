use crate::m002_user::User;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Media::Table)
                    .if_not_exists()
                    .col(string(Media::Id).primary_key())
                    .col(string(Media::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_id")
                            .from(Media::Table, Media::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(string_null(Media::PreviewId))
                    .col(string(Media::Hash))
                    .col(big_integer(Media::CreatedAt))
                    .col(big_integer(Media::LastModifiedAt))
                    .col(boolean(Media::Deleted))
                    .col(big_integer(Media::FileSize))
                    .col(string(Media::FileName))
                    .col(double_null(Media::Longitude))
                    .col(double_null(Media::Latitude))
                    .col(integer_null(Media::ImageWidth))
                    .col(integer_null(Media::ImageLength))
                    .col(string_null(Media::Make))
                    .col(string_null(Media::Model))
                    .col(string_null(Media::Fnumber))
                    .col(string_null(Media::ExposureTime))
                    .col(string_null(Media::PhotographicSensitivity))
                    .col(integer_null(Media::Orientation))
                    .col(
                        ColumnDef::new_with_type(
                            Media::ClipEmbeddings,
                            ColumnType::Vector(Some(512)),
                        )
                        .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Media::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Media {
    Table,
    Id,
    UserId,
    PreviewId,
    Hash,
    CreatedAt,
    LastModifiedAt,
    Deleted,
    FileSize,
    FileName,
    Longitude,
    Latitude,
    ImageWidth,
    ImageLength,
    Make,
    Model,
    Fnumber,
    ExposureTime,
    PhotographicSensitivity,
    Orientation,
    ClipEmbeddings,
}

use crate::m003_media::Media;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FaceData::Table)
                    .if_not_exists()
                    .col(string(FaceData::Id).primary_key())
                    .col(string(FaceData::MediaId).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("media_id")
                            .from(FaceData::Table, FaceData::MediaId)
                            .to(Media::Table, Media::Id),
                    )
                    .col(ColumnDef::new(FaceData::Embedding).vector(512).not_null()) 
                    .col(ColumnDef::new(FaceData::Coordinates).custom("POINT").not_null())
                    .col(integer_null(FaceData::ClusterId))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FaceData::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum FaceData {
    Table,
    Id,
    MediaId,
    Embedding,
    Coordinates,
    ClusterId,
}

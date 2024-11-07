use crate::{m003_media::Media, m004_face::Face};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        print!("{}",
                Table::create()
                    .table(FaceCluster::Table)
                    .if_not_exists()
                    .col(integer(FaceCluster::Id).primary_key())
                    .col(string(FaceCluster::MediaId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("media_id")
                            .from(FaceCluster::Table, FaceCluster::MediaId)
                            .to(Media::Table, Media::Id),
                    )
                    .col(ColumnDef::new_with_type(FaceCluster::Embedding,ColumnType::Vector(Some(512))).not_null())
                    .col(ColumnDef::new_with_type(FaceCluster::FaceBoundingBox,ColumnType::Vector(Some(4))).not_null())
                    .col(integer_null(FaceCluster::FaceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("face_id")
                            .from(FaceCluster::Table, FaceCluster::FaceId)
                            .to(Face::Table, Face::Id),
                    )
                    .to_owned().build(PostgresQueryBuilder));
        manager
            .create_table(
                Table::create()
                    .table(FaceCluster::Table)
                    .if_not_exists()
                    .col(integer(FaceCluster::Id).primary_key())
                    .col(string(FaceCluster::MediaId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("media_id")
                            .from(FaceCluster::Table, FaceCluster::MediaId)
                            .to(Media::Table, Media::Id),
                    )
                    .col(ColumnDef::new_with_type(FaceCluster::Embedding,ColumnType::Vector(Some(512))).not_null())
                    .col(ColumnDef::new_with_type(FaceCluster::FaceBoundingBox,ColumnType::Vector(Some(4))).not_null())
                    .col(integer_null(FaceCluster::FaceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("face_id")
                            .from(FaceCluster::Table, FaceCluster::FaceId)
                            .to(Face::Table, Face::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FaceCluster::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum FaceCluster {
    Table,
    Id,
    MediaId,
    Embedding,
    FaceBoundingBox,
    FaceId
}

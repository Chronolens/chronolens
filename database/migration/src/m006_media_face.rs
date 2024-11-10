use crate::{m003_media::Media, m005_cluster::Cluster};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MediaFace::Table)
                    .if_not_exists()
                    .col(integer(MediaFace::Id).primary_key().auto_increment())
                    .col(string(MediaFace::MediaId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("media_id")
                            .from(MediaFace::Table, MediaFace::MediaId)
                            .to(Media::Table, Media::Id),
                    )
                    .col(
                        ColumnDef::new_with_type(
                            MediaFace::Embedding,
                            ColumnType::Vector(Some(512)),
                        )
                        .not_null(),
                    )
                    .col(
                        ColumnDef::new_with_type(
                            MediaFace::FaceBoundingBox,
                            ColumnType::Vector(Some(4)),
                        )
                        .not_null(),
                    )
                    .col(integer(MediaFace::ClusterId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("cluster_id")
                            .from(MediaFace::Table, MediaFace::ClusterId)
                            .to(Cluster::Table, Cluster::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MediaFace::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MediaFace {
    Table,
    Id,
    MediaId,
    Embedding,
    FaceBoundingBox,
    ClusterId,
}

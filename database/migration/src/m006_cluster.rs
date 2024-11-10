use crate::m004_face::Face;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Cluster::Table)
                    .if_not_exists()
                    .col(integer(Cluster::Id).primary_key().auto_increment())
                    .col(string(Cluster::FaceId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("face_id")
                            .from(Cluster::Table, Cluster::FaceId)
                            .to(Face::Table, Face::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Cluster::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Cluster {
    Table,
    Id,
    FaceId
}

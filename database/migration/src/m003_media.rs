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
                    .col(date_time(Media::CreatedAt))
                    .col(string(Media::Hash))
                    .col(integer_null(Media::Duration))
                    .col(
                        ColumnDef::new(Media::Type)
                            .char_len(1)
                            .not_null()
                            .check(Expr::col(Media::Type).is_in(["i","v"]))
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
enum Media {
    Table,
    Id,
    UserId,
    PreviewId,
    Type,
    CreatedAt,
    Hash,
    Duration,
}

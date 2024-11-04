use sea_orm_migration::{prelude::*, schema::*};

use crate::m002_user::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Log::Table)
                    .if_not_exists()
                    .col(integer_uniq(Log::Id).primary_key().auto_increment())
                    .col(string(Log::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_id")
                            .from(Log::Table, Log::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(string(Log::Severity))
                    .col(date_time(Log::Date))
                    .col(string(Log::Message))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Log::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Log {
    Table,
    Id,
    UserId,
    Severity,
    Date,
    Message,
}

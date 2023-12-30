use sea_orm_migration::prelude::*;
use super::m20231228_182440_create_holder_table::Holder;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Credential::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Credential::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Credential::HolderId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-credential-holder_id")
                            .from(Credential::Table, Credential::HolderId)
                            .to(Holder::Table, Holder::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Credential::Details).json().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Credential::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Credential {
    Table,
    Id,
    HolderId,
    Details,
}

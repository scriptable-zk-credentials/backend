use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Holder::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Holder::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Holder::FirstName).string().not_null())
                    .col(ColumnDef::new(Holder::LastName).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Holder::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Holder {
    Table,
    Id,
    FirstName,
    LastName,
}

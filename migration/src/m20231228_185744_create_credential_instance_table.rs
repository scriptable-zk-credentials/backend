use sea_orm_migration::prelude::*;
use super::m20231228_183743_create_credential_table::Credential;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CredentialInstance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CredentialInstance::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CredentialInstance::CredentialId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-credential_instance-credential_id")
                            .from(CredentialInstance::Table, CredentialInstance::CredentialId)
                            .to(Credential::Table, Credential::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(CredentialInstance::Data).text().not_null())
                    .col(ColumnDef::new(CredentialInstance::Hash).text().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CredentialInstance::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CredentialInstance {
    Table,
    Id,
    CredentialId,
    Data,
    Hash,
}

pub use sea_orm_migration::prelude::*;

mod m20231228_182440_create_holder_table;
mod m20231228_183743_create_credential_table;
mod m20231228_185744_create_credential_instance_table;


pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231228_182440_create_holder_table::Migration),
            Box::new(m20231228_183743_create_credential_table::Migration),
            Box::new(m20231228_185744_create_credential_instance_table::Migration),
        ]
    }
}

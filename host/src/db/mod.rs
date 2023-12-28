use sea_orm::{DbConn, Database};
use migration::{Migrator, MigratorTrait};


pub async fn db_start() -> DbConn {
    let db = Database::connect("sqlite://credentials.sqlite?mode=rwc").await
        .expect("failed to connect to create DB connection");
    assert!(db.ping().await.is_ok());
    
    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations for tests");

    db
}
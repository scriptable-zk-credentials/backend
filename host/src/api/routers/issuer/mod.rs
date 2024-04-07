mod holders;
mod credentials;
mod credential_instances;
mod schemas;

use std::sync::Arc;
use holders::holders_router;
use credentials::credentials_router;
use credential_instances::instances_router;
use schemas::schemas_router;
use sea_orm::DbConn;
use axum::routing::Router;
use serde::{Serialize, Deserialize};
use shared::types::ZkCommit;

use crate::adapters::RegistryContract;


#[derive(Serialize)]
pub struct CheckZkpResponse {
    verdict: bool,
    error: Option<String>,
    journal: Option<ZkCommit>,
}


pub fn issuer_router(db_connection: DbConn, registry: Arc<RegistryContract>) -> Router {
    Router::new()
        .nest("/schemas", schemas_router(Arc::clone(&registry)))
        .nest("/holders", holders_router(db_connection.clone()))
        .nest("/credentials", credentials_router(db_connection.clone()))
        .nest("/instances", instances_router(db_connection.clone(), Arc::clone(&registry)))
}

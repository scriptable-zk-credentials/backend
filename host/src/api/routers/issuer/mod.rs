mod holders;
mod credentials;
mod credential_instances;
mod schemas;

use std::sync::Arc;
use holders::holders_router;
use credentials::credentials_router;
use credential_instances::instances_router;
use schemas::schemas_router;
use methods::{ZK_PROVER_ELF, ZK_PROVER_ID};
use std::time::Instant;
use sea_orm::DbConn;
use axum::{
    routing::{Router, post, get},
    http::StatusCode, Json,
};
use risc0_zkvm::{
    Receipt,
    serde::from_slice,
};
use serde::{Serialize, Deserialize};
use shared::types::ZkCommit;
use base64ct::{Base64, Encoding};

use crate::adapters::RegistryContract;


#[derive(Deserialize)]
pub struct CheckZkpArgs {
    // Base64 ecncoded risc0 Receipt
    base64_receipt: String,
}

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

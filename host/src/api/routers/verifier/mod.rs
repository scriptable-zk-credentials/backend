use std::sync::Arc;
use methods::{ZK_PROVER_ELF, ZK_PROVER_ID};
use std::time::Instant;
use sea_orm::DbConn;
use axum::{
    routing::{Router, post, get},
    http::StatusCode, Json,
    extract::State,
};
use risc0_zkvm::{
    Receipt,
    serde::from_slice,
};
use serde::{Serialize, Deserialize};
use shared::types::ZkCommit;
use base64ct::{Base64, Encoding};
use tokio::join;

use crate::adapters::RegistryContract;


#[derive(Deserialize)]
pub struct CheckArgs {
    // list of credential issuers, in the same order of cred_hashes from the ZKP journal
    cred_issuers: Vec<String>,
    // Base64 ecncoded risc0 Receipt
    base64_receipt: String,
}

#[derive(Serialize)]
pub struct CheckResponse {
    verdict: bool,
    error: Option<String>,
    journal: Option<ZkCommit>,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
    registry: Arc<RegistryContract>,
}


pub fn verifier_router(db_connection: DbConn, registry: Arc<RegistryContract>) -> Router {
    let state = AppState { db_connection, registry, };
    
    Router::new()
        //.route("/check-script", post(gen_js_proof))
        .route("/check", post(check_presentation))
        .with_state(state)
}


// Check a verifiable credential presentation submitted by a user
// 1) Check the ZKP, and parse the journal (we do this first as it takes the least amount of time)
// 2) Check that provided credentials are in the issuer's registry contract
// 3) fetch the schema, make sure the schema matches stuff in the script
async fn check_presentation(
    State(state): State<AppState>,
    Json(payload): Json<CheckArgs>
) -> (StatusCode, Json<CheckResponse>) {
    let receipt: Receipt = bincode::deserialize(&Base64::decode_vec(&payload.base64_receipt).unwrap()).unwrap();
    
    // Measure ZKP verification time
    let start_time = Instant::now();
    // Verify ZKP
    let (verdict, error, journal) = match receipt.verify(ZK_PROVER_ID) {
        Ok(()) => {
            println!("ZKP verification time: {:?}", start_time.elapsed());
            // parse the ZKP journal
            let journal: ZkCommit = from_slice(&receipt.journal.bytes).unwrap();
            // check that all vectors containing credential information have the same length
            if journal.cred_hashes.len() != journal.cred_schemas.len() || journal.cred_hashes.len() != payload.cred_issuers.len() {
                (
                    false,
                    Option::Some("Vectors containing credential information must be of the same length".to_string()),
                    Option::None
                )
            }
            else {
                // Concurrently request 2 things from the regitry contract (to lower latency)
                // 1- make sure all presented credential hashes are in the registry contract
                // 2- fetch all credential schemas
                let (registry_checks, schemas) = join!(
                    state.registry.check_credentials(journal.cred_hashes
                        .iter()
                        .enumerate()
                        .map(|(i, cred_hash)| (payload.cred_issuers[i].clone(), cred_hash.clone()))
                        .collect()
                    ),
                    state.registry.get_schemas(journal.cred_schemas
                        .iter()
                        .enumerate()
                        .map(|(i, &schema_id)| (payload.cred_issuers[i].clone(), schema_id))
                        .collect()
                    )
                );

                if registry_checks.contains(&false) {
                    (
                        false,
                        Option::Some("Some provided credential is not valid for the given issuer on the registry contract".to_string()),
                        Option::None
                    )
                }
                else if schemas.contains(&"".to_string()) {
                    (
                        false,
                        Option::Some("Some provided schema_id is not valid for the given issuer on the registry contract".to_string()),
                        Option::None
                    )
                }
                else {
                    (true, Option::None, Option::Some(journal))
                }
            }
        },
        Err(error) => {
            (false, Option::Some(error.to_string()), Option::None)
        },
    };

    (
        StatusCode::ACCEPTED,
        Json(CheckResponse { verdict, error, journal }),
    )
}

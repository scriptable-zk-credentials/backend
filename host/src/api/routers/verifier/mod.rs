use std::sync::{Arc, Mutex};
use methods::{ZK_PROVER_ELF, ZK_PROVER_ID};
use shared::types::ScriptLang;
use std::time::Instant;
use sea_orm::DbConn;
use axum::{
    extract::State, http::StatusCode, routing::{get, post, Router}, Json
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


#[derive(Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub enum RequestStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Deserialize)]
pub struct ModifyRequestsArgs { 
    approve: Vec<usize>,
    deny: Vec<usize>,
}

#[derive(Serialize, Clone)]
pub struct Request {
    pub status: RequestStatus,
    pub cred_hashes: Vec<String>,
    pub cred_schemas: Vec<String>,
    pub lang: ScriptLang,
    pub script: String,
    pub result: bool,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
    registry: Arc<RegistryContract>,
    requests: Arc<Mutex<Vec<Request>>>,
}


pub fn verifier_router(db_connection: DbConn, registry: Arc<RegistryContract>) -> Router {
    let app_state = AppState {
        db_connection,
        registry,
        requests: Arc::new(Mutex::new(Vec::new())),
    };

    
    Router::new()
        //.route("/check-script", post(gen_js_proof))
        .route("/check", post(check_presentation))
        .route("/presentations", get(get_presentations).post(modify_presentations))
        .with_state(app_state)
}


// Check a verifiable credential presentation submitted by a user
// 1) Check the ZKP, and parse the journal (we do this first as it takes the least amount of time)
// 2) Check that provided credentials are in the issuer's registry contract
// 3) fetch the schema, make sure the schema matches stuff in the script
pub async fn check_presentation(
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

                // credential error
                if registry_checks.contains(&false) {
                    (
                        false,
                        Option::Some("Some provided credential is not valid for the given issuer on the registry contract".to_string()),
                        Option::None
                    )
                }
                // schema error
                else if schemas.contains(&"".to_string()) {
                    (
                        false,
                        Option::Some("Some provided schema_id is not valid for the given issuer on the registry contract".to_string()),
                        Option::None
                    )
                }
                else {
                    // if ZK-Program did not have errors, add to pending requests
                    if !(&journal.has_error) {
                        let journal_clone = journal.clone();
                        let mut requests = state.requests.lock().expect("mutex was poisoned");
                        requests.push(Request {
                            status: RequestStatus::Pending,
                            cred_hashes: journal_clone.cred_hashes,
                            cred_schemas: schemas,
                            lang: journal_clone.lang,
                            script: journal_clone.script,
                            result: journal_clone.result,
                        });
                    }

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

// get pending requests
pub async fn get_presentations(State(state): State<AppState>) -> (StatusCode, Json<Vec<Request>>) {
    let requests = {
        let requests = state.requests.lock().expect("mutex was poisoned");
        requests.clone()
    };

    (StatusCode::ACCEPTED, Json(requests))
}

pub async fn modify_presentations(
    State(state): State<AppState>,
    Json(payload): Json<ModifyRequestsArgs>,
) -> (StatusCode, Json<bool>) {
    {
        let mut requests = state.requests.lock().expect("mutex was poisoned");
        payload.approve
            .iter()
            .for_each(|&req_id| {
                if req_id < requests.len() {
                    requests[req_id].status = RequestStatus::Approved;
                }
            });
        payload.deny
            .iter()
            .for_each(|&req_id| {
                if req_id < requests.len() {
                    requests[req_id].status = RequestStatus::Denied;
                }
            });

    };

    (StatusCode::ACCEPTED, Json(true))
}
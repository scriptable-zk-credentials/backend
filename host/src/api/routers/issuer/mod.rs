mod holders;
mod credentials;

use holders::holders_router;
use credentials::credentials_router;
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


pub fn issuer_router(db_connection: DbConn) -> Router {
    Router::new()
        //.route("/check-script", post(gen_js_proof))
        .route("/check-zkp", post(check_zkp))
        .nest("/holders", holders_router(db_connection.clone()))
        .nest("/credentials", credentials_router(db_connection.clone()))
}


// Verify ZK-Proofs
async fn check_zkp(Json(payload): Json<CheckZkpArgs>) -> (StatusCode, Json<CheckZkpResponse>) {
    let start_time = Instant::now();

    let receipt: Receipt = bincode::deserialize(&Base64::decode_vec(&payload.base64_receipt).unwrap()).unwrap();
    let (verdict, error, journal) = match receipt.verify(ZK_PROVER_ID) {
        Ok(()) => {
            let journal: ZkCommit = from_slice(&receipt.journal.bytes).unwrap();
            (true, Option::None, Option::Some(journal))
        },
        Err(error) => (false, Option::Some(error.to_string()), Option::None),
    };

    println!("Verifier duration {:?}", start_time.elapsed());

    (
        StatusCode::ACCEPTED,
        Json(CheckZkpResponse { verdict, error, journal }),
    )
}

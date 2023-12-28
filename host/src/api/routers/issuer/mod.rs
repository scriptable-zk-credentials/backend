use methods::{ZK_PROVER_ELF, ZK_PROVER_ID};
use entity::{holder, credential, credential_instance};
use std::time::Instant;
use sea_orm::{DbConn, EntityTrait, ActiveModelTrait, Set, QueryFilter};
use axum::{
    routing::{Router, post, get, delete},
    http::StatusCode, Json,
    extract::{State, Path},
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

#[derive(Deserialize)]
pub struct ModifyHoldersArgs {
    pub add: Vec<HolderInfo>,
    pub remove: Vec<i32>,
}

#[derive(Deserialize)]
struct HolderInfo {
    pub first_name: String,
    pub last_name: String,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
}

pub fn issuer_router(db_connection: DbConn) -> Router {
    let state = AppState { db_connection, };
    
    Router::new()
        //.route("/check-script", post(gen_js_proof))
        .route("/check-zkp", post(check_zkp))
        .route("/holders/:holder_id", get(get_holders))
        .route("/holders", post(modify_holders))
        .with_state(state)
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

#[axum::debug_handler]
pub async fn get_holders(
    State(state): State<AppState>,
) -> (StatusCode, Json<Vec<holder::Model>>) {
    let holders = holder::Entity::find().all(&state.db_connection).await.expect("failed to get post from DB");
    
    (StatusCode::ACCEPTED, Json(holders))
}

pub async fn modify_holders(
    State(state): State<AppState>,
    Json(payload): Json<ModifyHoldersArgs>,
) -> (StatusCode, Json<bool>) {
    let new_holders: Vec<holder::ActiveModel> = payload.add
        .iter()
        .map(|holder_info| holder::ActiveModel {
            first_name: Set(holder_info.first_name),
            last_name: Set(holder_info.last_name),
            ..Default::default()
        })
        .collect();

    holder::Entity::insert_many(new_holders).exec(&state.db_connection).await.expect("failed to insert new holders in DB");
    holder::Entity::delete_many().filter(holder::Column::Id.into::<i32>()).exec(&state.db_connection).await.expect("failed to insert new holders in DB");

    (StatusCode::ACCEPTED, Json(true))
}

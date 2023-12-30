use entity::holder;
use sea_orm::{DbConn, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait};
use axum::{
    routing::{Router, post, get},
    http::StatusCode, Json,
    extract::{State, Path},
};
use serde::{Serialize, Deserialize};
use shared::types::ZkCommit;


#[derive(Deserialize)]
pub struct ModifyHoldersArgs {
    pub remove: Vec<i32>,
    pub add: Vec<HolderInfo>,
}

#[derive(Deserialize)]
pub struct HolderInfo {
    pub first_name: String,
    pub last_name: String,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
}

pub fn holders_router(db_connection: DbConn) -> Router {
    let state = AppState { db_connection, };
    
    Router::new()
        .route(
            "/",
            get(get_holders).post(modify_holders)
        )
        .with_state(state)
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
            first_name: Set(holder_info.first_name.clone()),
            last_name: Set(holder_info.last_name.clone()),
            ..Default::default()
        })
        .collect();

    // Remove holders
    if !payload.remove.is_empty() {
        holder::Entity::delete_many()
            .filter(holder::Column::Id.is_in(payload.remove))
            .exec(&state.db_connection)
            .await.expect("failed to remove holders from DB");
    }
    // Add holders
    if !new_holders.is_empty() {
        holder::Entity::insert_many(new_holders)
            .exec(&state.db_connection)
            .await.expect("failed to insert new holders in DB");
    }

    (StatusCode::ACCEPTED, Json(true))
}

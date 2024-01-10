use std::sync::Arc;
use entity::credential;
use sea_orm::{DbConn, EntityTrait, Set, QueryFilter, ColumnTrait};
use axum::{
    routing::{Router, post, get},
    http::StatusCode, Json,
    extract::{State, Path},
};
use serde::Deserialize;
use shared::types::SchemaId;

use crate::adapters::RegistryContract;


#[derive(Deserialize)]
pub struct ModifyCredentialsArgs {
    pub holder_id: u32,
    // vector of credential IDs to be removed for this user
    pub remove: Vec<u32>,
    // vector of credential details (JSON strings) to be added for this user
    pub add: Vec<(SchemaId, String)>,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
    registry: Arc<RegistryContract>,
}

pub fn credentials_router(db_connection: DbConn, registry: Arc<RegistryContract>) -> Router {
    let state = AppState { db_connection, registry, };
    
    Router::new()
        .route("/", post(modify_credentials))
        .route("/:holder_id",get(get_credentials))
        .with_state(state)
}

#[axum::debug_handler]
pub async fn get_credentials(
    State(state): State<AppState>,
    Path(holder_id): Path<u32>,
) -> (StatusCode, Json<Vec<credential::Model>>) {
    let credentials = credential::Entity::find()
        .filter(credential::Column::HolderId.eq(holder_id))
        .all(&state.db_connection)
        .await.expect("failed to get holder credentials from DB");
    
    (StatusCode::ACCEPTED, Json(credentials))
}

pub async fn modify_credentials(
    State(state): State<AppState>,
    Json(payload): Json<Vec<ModifyCredentialsArgs>>,
) -> (StatusCode, Json<bool>) {
    // group all credentials to be removed into: Vec<credential_id>
    let to_remove: Vec<u32> = payload
        .iter()
        .map(|ModifyCredentialsArgs { remove, .. }| remove)
        .flatten()
        .copied()
        .collect();
    // group all credentials to be added into: Vec<(holder_id, credential_details)>
    let to_add: Vec<(u32, u32, String)> = payload
        .iter()
        .map(|ModifyCredentialsArgs { holder_id, add, .. }| {
            add.iter().map(|(schema_id, details)| (*holder_id, *schema_id, details.clone()))
        })
        .flatten()
        .collect();

    // Remove credentials
    if !to_remove.is_empty() {
        credential::Entity::delete_many()
            .filter(credential::Column::Id.is_in(to_remove))
            .exec(&state.db_connection)
            .await.expect("failed to remove credentials from DB");
    }
    // Add credentials
    if !to_add.is_empty() {
        // create new credential objects
        let new_credentials: Vec<credential::ActiveModel> = to_add
        .iter()
        .map(|(holder_id, schema_id, credential_details)| credential::ActiveModel {
            holder_id: Set(*holder_id),
            schema_id: Set(*schema_id),
            details: Set(credential_details.clone()),
            ..Default::default()
        })
        .collect();
        // insert new credentials in DB
        credential::Entity::insert_many(new_credentials)
            .exec(&state.db_connection)
            .await.expect("failed to insert new credentials in DB");
    }

    (StatusCode::ACCEPTED, Json(true))
}

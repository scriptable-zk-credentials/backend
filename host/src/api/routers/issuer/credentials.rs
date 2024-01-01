use entity::credential;
use sea_orm::{DbConn, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait};
use axum::{
    routing::{Router, post, get},
    http::StatusCode, Json,
    extract::{State, Path},
};
use serde::{Serialize, Deserialize};


#[derive(Deserialize)]
pub struct ModifyCredentialsArgs {
    pub holder_id: i32,
    // vector of credential IDs to be removed for this user
    pub remove: Vec<i32>,
    // vector of credential details (JSON strings) to be added for this user
    pub add: Vec<String>,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
}

pub fn credentials_router(db_connection: DbConn) -> Router {
    let state = AppState { db_connection, };
    
    Router::new()
        .route("/", post(modify_credentials))
        .route("/:holder_id",get(get_credentials))
        .with_state(state)
}

#[axum::debug_handler]
pub async fn get_credentials(
    State(state): State<AppState>,
    Path(holder_id): Path<i32>,
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
    let to_remove: Vec<i32> = payload
        .iter()
        .map(|ModifyCredentialsArgs { remove, .. }| remove)
        .flatten()
        .copied()
        .collect();
    // group all credentials to be added into: Vec<(holder_id, credential_details)>
    let to_add: Vec<(i32, String)> = payload
        .iter()
        .map(|ModifyCredentialsArgs { holder_id, add, .. }| {
            add.iter().map(|cred_details| (holder_id.clone(), cred_details.clone()))
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
        .map(|(holder_id, credential_details)| credential::ActiveModel {
            holder_id: Set(holder_id.clone()),
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

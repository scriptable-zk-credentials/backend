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
    pub remove: Vec<i32>,
    pub add: Vec<CredentialInfo>,
}

#[derive(Deserialize)]
pub struct CredentialInfo {
    pub holder_id: i32,
    pub details: String,
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
    Json(payload): Json<ModifyCredentialsArgs>,
) -> (StatusCode, Json<bool>) {
    let new_credentials: Vec<credential::ActiveModel> = payload.add
        .iter()
        .map(|credential_info| credential::ActiveModel {
            holder_id: Set(credential_info.holder_id.clone()),
            details: Set(credential_info.details.clone()),
            ..Default::default()
        })
        .collect();

    // Remove credentials
    if !payload.remove.is_empty() {
        credential::Entity::delete_many()
            .filter(credential::Column::Id.is_in(payload.remove))
            .exec(&state.db_connection)
            .await.expect("failed to remove credentials from DB");
    }
    // Add credentials
    if !new_credentials.is_empty() {
        credential::Entity::insert_many(new_credentials)
            .exec(&state.db_connection)
            .await.expect("failed to insert new credentials in DB");
    }

    (StatusCode::ACCEPTED, Json(true))
}

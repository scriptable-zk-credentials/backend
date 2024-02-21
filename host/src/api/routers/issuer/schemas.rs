use std::sync::Arc;
use axum::{
    routing::{Router, get},
    http::StatusCode, Json,
    extract::State,
};
use serde::{Serialize, Deserialize};

use crate::adapters::RegistryContract;


#[derive(Deserialize)]
pub struct AddSchemaArgs {
    pub schema: String,
}

#[derive(Clone)]
pub struct AppState {
    registry: Arc<RegistryContract>,
}

pub fn schemas_router(registry: Arc<RegistryContract>) -> Router {
    let state = AppState { registry, };
    
    Router::new()
        .route(
            "/",
            get(get_schemas).post(add_schema)
        )
        .with_state(state)
}

#[axum::debug_handler]
pub async fn get_schemas(
    State(state): State<AppState>,
) -> (StatusCode, Json<Vec<String>>) {
    let holders = state.registry.get_issuer_schemas().await;
    
    (StatusCode::ACCEPTED, Json(holders))
}

pub async fn add_schema(
    State(state): State<AppState>,
    Json(payload): Json<AddSchemaArgs>,
) -> (StatusCode, Json<bool>) {
    state.registry.add_schema(payload.schema).await;

    (StatusCode::ACCEPTED, Json(true))
}

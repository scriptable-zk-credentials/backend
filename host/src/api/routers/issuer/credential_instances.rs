use std::{sync::Arc, collections::HashSet};
use entity::{credential, credential_instance};
use sea_orm::{DbConn, EntityTrait, Set, QueryFilter, ColumnTrait, Condition, QuerySelect, FromQueryResult};
use axum::{
    routing::{Router, get, post},
    http::StatusCode, Json,
    extract::{State, Path},
};
use sha2::{Sha256, Digest};
use base64ct::{Base64, Encoding};
use serde::Deserialize;
use serde_json::{to_string, from_str, Value};

use crate::adapters::RegistryContract;


#[derive(Deserialize)]
pub struct ModifyInstancesArgs {
    pub remove: Vec<u32>,
    pub num_to_add: usize,
}

#[derive(FromQueryResult)]
struct InstanceHash {
    hash: String,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
    registry: Arc<RegistryContract>,
}

pub fn instances_router(db_connection: DbConn, registry: Arc<RegistryContract>) -> Router {
    let state = AppState { db_connection, registry, };
    
    Router::new()
        .route(
            "/:credential_id",
            get(get_instances).post(modify_instances)
        )
        .route("/sync", post(sync_instances))
        .with_state(state)
}

#[axum::debug_handler]
pub async fn get_instances(
    State(state): State<AppState>,
    Path(credential_id): Path<u32>,
) -> (StatusCode, Json<Vec<credential_instance::Model>>) {
    let instances = credential_instance::Entity::find()
        .filter(credential_instance::Column::CredentialId.eq(credential_id))
        .all(&state.db_connection)
        .await.expect("failed to get credential instances from DB");
    
    (StatusCode::ACCEPTED, Json(instances))
}

pub async fn modify_instances(
    State(state): State<AppState>,
    Path(credential_id): Path<u32>,
    Json(payload): Json<ModifyInstancesArgs>,
) -> (StatusCode, Json<bool>) {
    // Remove credential instances
    if !payload.remove.is_empty() {
        credential_instance::Entity::delete_many()
            .filter(
                Condition::all()
                    .add(credential_instance::Column::Id.is_in(payload.remove))
                    .add(credential_instance::Column::CredentialId.eq(credential_id))
            )
            .exec(&state.db_connection)
            .await.expect("failed to remove credentials from DB");
    }
    // Add credential instances
    if payload.num_to_add > 0 {
        // get parent credential from DB
        let maybe_credential = credential::Entity::find_by_id(credential_id)
            .one(&state.db_connection)
            .await.expect("failed to get credential for given ID");
        // check if the DB has such a credential
        match maybe_credential {
            Some(credential) => {
                // Credential details are stringified JSON. Try parsing them as JSON Object
                let cred_details: Result<Value, _> = from_str(&credential.details);
                match cred_details {
                    Ok(Value::Object(details)) => {
                        // all credential instances inherit details from the "parent" credential
                        // instances are distinguished by the nonce attribute. We add it later
                        let mut shared_details = details;
                        let mut new_instances: Vec<credential_instance::ActiveModel> = Vec::with_capacity(payload.num_to_add);
                        for _i in 0..payload.num_to_add {
                            // add unique random nonce to each copy to distinguish their hashes
                            shared_details.insert(
                                "nonce".to_string(),
                                // save the nonce nunber in base64
                                // shorter representation => faster parsing inside zkVM
                                Value::from(Base64::encode_string(&rand::random::<u128>().to_ne_bytes()))
                            );
                            // add SchemaId
                            shared_details.insert("schema_id".to_string(), credential.schema_id.into());
                            // obtain a stringified JSON representation of the credential instance
                            let details_str = to_string(&shared_details).unwrap();
                            new_instances.push(credential_instance::ActiveModel {
                                credential_id: Set(credential_id),
                                data: Set(details_str.clone()),
                                hash: Set(Base64::encode_string(&Sha256::digest(details_str))),
                                ..Default::default()
                            });
                        }
                        // insert new instances in DB
                        credential_instance::Entity::insert_many(new_instances)
                        .exec(&state.db_connection)
                        .await.expect("failed to insert new credentials in DB");
                    },
                    _ => {},
                }
            },
            None => {},
        }
    }

    (StatusCode::ACCEPTED, Json(true))
}

// sync credential hashes on the registry contract to reflect the state of the issuer DB
pub async fn sync_instances(State(state): State<AppState>) -> (StatusCode, Json<bool>) {
    // get all credential hashes on the DB
    let db_hashes: HashSet<String> = credential_instance::Entity::find()
        .select_only()
        .column(credential_instance::Column::Hash)
        .into_model::<InstanceHash>()
        .all(&state.db_connection)
        .await
        .unwrap()
        .into_iter()
        .map(|q_result| q_result.hash)
        .collect();
    // get all credentials hashes on the registry contract
    let registry_hashes: HashSet<String> = state.registry.get_credentials().await.into_iter().collect();

    state.registry.modify_credentials(
        registry_hashes.difference(&db_hashes).into_iter().cloned().collect(),
        db_hashes.difference(&registry_hashes).into_iter().cloned().collect(),
    ).await;

    (StatusCode::ACCEPTED, Json(true))
}

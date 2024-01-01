use entity::{credential, credential_instance};
use sea_orm::{DbConn, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait, Condition};
use axum::{
    routing::{Router, post, get},
    http::StatusCode, Json,
    extract::{State, Path},
};
use sha2::{Sha256, Digest};
use base64ct::{Base64, Encoding};
use serde::{Serialize, Deserialize};
use serde_json::{to_string, from_str, Value};


#[derive(Deserialize)]
pub struct ModifyInstancesArgs {
    pub remove: Vec<i32>,
    pub num_to_add: usize,
}

#[derive(Clone)]
pub struct AppState {
    db_connection: DbConn,
}

pub fn instances_router(db_connection: DbConn) -> Router {
    let state = AppState { db_connection, };
    
    Router::new()
        .route(
            "/:credential_id",
            get(get_instances).post(modify_instances)
        )
        .with_state(state)
}

#[axum::debug_handler]
pub async fn get_instances(
    State(state): State<AppState>,
    Path(credential_id): Path<i32>,
) -> (StatusCode, Json<Vec<credential_instance::Model>>) {
    let instances = credential_instance::Entity::find()
        .filter(credential_instance::Column::CredentialId.eq(credential_id))
        .all(&state.db_connection)
        .await.expect("failed to get credential instances from DB");
    
    (StatusCode::ACCEPTED, Json(instances))
}

pub async fn modify_instances(
    State(state): State<AppState>,
    Path(credential_id): Path<i32>,
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

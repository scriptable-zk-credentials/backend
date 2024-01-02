use near_sdk::{
    borsh::{self, BorshSerialize, BorshDeserialize},
    store::{LookupMap, LookupSet, Vector},
    BorshStorageKey, AccountId, env,
    near_bindgen, CryptoHash,
};


#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    IssuerSchemasMap,
    IssuerSchemasVector { issuer_account_id_hash: CryptoHash },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    // Map: Issuer => Schemas
    // nested collections. See: https://docs.near.org/sdk/rust/contract-structure/nesting
    issuer_schemas: LookupMap<AccountId, Vector<String>>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            issuer_schemas: LookupMap::new(StorageKey::IssuerSchemasMap),
        }
    }
}


#[near_bindgen]
impl Contract {

    /// returns SchemaId of the added schema (which is also its index in the vector)
    #[payable]
    pub fn add_schema(&mut self, schema: String) -> u32 {
        let issuer = env::predecessor_account_id();
        let result: u32 = match self.issuer_schemas.get_mut(&issuer) {
            // issuer has schemas registered before 
            Some(existing_schemas) => {
                // add the new schema
                existing_schemas.push(schema);
                // return the schemaId (index of the newly added schema in the array)
                existing_schemas.len() - 1
            },
            // issuer will register their first schema
            None => {
                let mut new_vec = Vector::new(StorageKey::IssuerSchemasVector {
                    issuer_account_id_hash: env::sha256_array(issuer.as_bytes()),
                });
                // add the new schema
                new_vec.push(schema);
                // save the new schemas Vector into the Map: Issuer => schemas
                self.issuer_schemas.set(issuer, Some(new_vec));
                // the newly added schema has index 0
                0
            }
        };
        
        result
    }

    /// supports pagination using from and limit
    pub fn get_issuer_schemas(&self, issuer: AccountId, from: Option<u32>, limit: Option<u32>) -> Vec<String> {
        let maybe_schemas = self.issuer_schemas.get(&issuer);
        let result: Vec<String> = match maybe_schemas {
            None => Vec::new(),
            Some(all_schemas) => {
                // if no start index specified, use 0
                let inn_from = match from {
                    Some(from_idx) => from_idx,
                    _ => 0,
                };
                // if no limit specified, get all the issuer's schemas
                let inn_limit = match limit {
                    Some(chosen_limit) => chosen_limit,
                    _ => all_schemas.len(),
                };

                all_schemas
                    .iter()
                    .skip(inn_from.try_into().unwrap())
                    .take(inn_limit.try_into().unwrap())
                    .cloned()
                    .collect()
            }
        };

        result
    }
}

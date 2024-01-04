use near_sdk::{
    borsh::{self, BorshSerialize, BorshDeserialize},
    store::{LookupMap, UnorderedSet, Vector},
    BorshStorageKey, AccountId, env,
    near_bindgen, CryptoHash, assert_one_yocto, require,
};
use shared::types::SchemaId;


#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    SchemasMap,
    SchemasVector { issuer_account_id_hash: CryptoHash },
    CredentialsMap,
    CredentialsSet { issuer_account_id_hash: CryptoHash },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    // CAUTION !! we use nested collections. See: https://docs.near.org/sdk/rust/contract-structure/nesting
    // Map: Issuer => Schemas
    schemas: LookupMap<AccountId, Vector<String>>,
    // Map: Issuer => credential hashes
    credentials: LookupMap<AccountId, UnorderedSet<String>>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            schemas: LookupMap::new(StorageKey::SchemasMap),
            credentials: LookupMap::new(StorageKey::CredentialsMap),
        }
    }
}


#[near_bindgen]
impl Contract {

    /// Returns SchemaId of the added schema (which is also its index in the vector)
    /// CAUTION!! Schemas are add-only
    #[payable]
    pub fn add_schema(&mut self, schema: String) -> SchemaId {
        assert_one_yocto();

        let issuer = env::predecessor_account_id();
        let result: SchemaId = match self.schemas.get_mut(&issuer) {
            // Issuer has schemas registered before 
            Some(existing_schemas) => {
                // Add the new schema
                existing_schemas.push(schema);
                // Return the schemaId (index of the newly added schema in the array)
                existing_schemas.len() - 1
            },
            // Issuer will register their first schema
            None => {
                let mut new_vec = Vector::new(StorageKey::SchemasVector {
                    issuer_account_id_hash: env::sha256_array(issuer.as_bytes()),
                });
                // Add the new schema
                new_vec.push(schema);
                // Save the new schemas Vector into the Map: Issuer => schemas
                self.schemas.set(issuer, Some(new_vec));
                // The newly added schema has index 0
                0
            }
        };
        
        result
    }

    /// Add and remove credential commitments
    #[payable]
    pub fn modify_credentials(&mut self, remove: Vec<String>, add: Vec<String>) {
        assert_one_yocto();

        let issuer = env::predecessor_account_id();
        match self.credentials.get_mut(&issuer) {
            // Issuer has committed credentials before 
            Some(existing_credentials) => {
                // Remove credentials
                remove.iter().for_each(|credential| { existing_credentials.remove(credential); });
                // Add the new credentials
                existing_credentials.extend(add);
            },
            // Issuer will commit credentials for the first time
            None => {
                let mut new_set = UnorderedSet::new(StorageKey::CredentialsSet {
                    issuer_account_id_hash: env::sha256_array(issuer.as_bytes()),
                });
                // Since we execute removals before additions, there is no point of running removals on an empty set
                require!(remove.is_empty(), "Removals are not allowed in the first commitment");
                // Add the new credentials
                new_set.extend(add);
                // Save the new schemas Vector into the Map: Issuer => schemas
                self.credentials.set(issuer, Some(new_set));
            }
        };
    }

    /// Supports pagination using from and limit
    pub fn get_schemas(&self, issuer: AccountId, from: Option<u32>, limit: Option<u32>) -> Vec<String> {
        let maybe_schemas = self.schemas.get(&issuer);
        let result: Vec<String> = match maybe_schemas {
            None => Vec::new(),
            Some(all_schemas) => {
                // If no start index specified, use 0
                let inn_from = match from {
                    Some(from_idx) => from_idx,
                    _ => 0,
                };
                // If no limit specified, get all the issuer's schemas
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

    /// Supports pagination using from and limit
    pub fn get_credentials(&self, issuer: AccountId, from: Option<u32>, limit: Option<u32>) -> Vec<String> {
        let maybe_credentials = self.credentials.get(&issuer);
        let result: Vec<String> = match maybe_credentials {
            None => Vec::new(),
            Some(all_credentials) => {
                // If no start index specified, use 0
                let inn_from = match from {
                    Some(from_idx) => from_idx,
                    _ => 0,
                };
                // If no limit specified, get all the issuer's schemas
                let inn_limit = match limit {
                    Some(chosen_limit) => chosen_limit,
                    _ => all_credentials.len(),
                };

                all_credentials
                    .iter()
                    .skip(inn_from.try_into().unwrap())
                    .take(inn_limit.try_into().unwrap())
                    .cloned()
                    .collect()
            }
        };

        result
    }

    pub fn has_credential(&self, issuer: AccountId, credential_hash: String) -> bool {
        let result = match self.credentials.get(&issuer) {
            None => false,
            Some(credentials) => credentials.contains(&credential_hash),
        };

        result
    }
}

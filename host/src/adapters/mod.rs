mod wallet;

use std::sync::Arc;
use near_primitives::{
    types::AccountId,
    transaction::{Action::FunctionCall, FunctionCallAction},
};
use serde_json::{json, to_vec};
use wallet::NearWallet;

pub struct RegistryContract {
    wallet: NearWallet,
    contract_address: AccountId,
}

impl RegistryContract {
    pub fn new() -> Self {
        let wallet = NearWallet::new();
        // read registry contract address from ENV file
        let contract_addr = std::env::var("REGISTRY_CONTRACT_ADDRESS").expect("REGISTRY_CONTRACT_ADDRESS must be set.");
        
        Self { wallet, contract_address: contract_addr.parse().unwrap() }
    }

    pub fn get_issuer_id(&self) -> String {
        self.wallet.signer_account_id()
    }

    pub async fn get_schemas(&self) -> Vec<String> {
        let result: Vec<String> = self.wallet.view(
            &self.contract_address,
            "get_schemas",
            json!({
                "issuer": self.get_issuer_id()
            })
        ).await.unwrap();

        result
    }

    pub async fn add_schema(&self, schema: String) {
        self.wallet.tx(
            &self.contract_address,
            vec![FunctionCall(FunctionCallAction {
                method_name: "add_schema".to_string(),
                args: to_vec(&json!({
                    "schema": schema,
                })).unwrap(),
                gas: 300_000_000_000_000,
                deposit: 1,
            })]
        ).await.unwrap();
    }

    pub async fn get_credentials(&self) -> Vec<String> {
        let result: Vec<String> = self.wallet.view(
            &self.contract_address,
            "get_credentials",
            json!({
                "issuer": self.get_issuer_id()
            })
        ).await.unwrap();

        result
    }

    pub async fn has_credential(&self, credential_hash: String) -> bool {
        let result: bool = self.wallet.view(
            &self.contract_address,
            "has_credential",
            json!({
                "issuer": self.get_issuer_id()
            })
        ).await.unwrap();

        result
    }

    pub async fn modify_credentials(&self, remove: Vec<String>, add: Vec<String>) {
        self.wallet.tx(
            &self.contract_address,
            vec![FunctionCall(FunctionCallAction {
                method_name: "modify_credentials".to_string(),
                args: to_vec(&json!({
                    "remove": remove,
                    "add": add,
                })).unwrap(),
                gas: 300_000_000_000_000,
                deposit: 1,
            })]
        ).await.unwrap();
    }
}
mod wallet;

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
        let near_env = std::env::var("NEAR_ENV").expect("NEAR_ENV must be set.").to_uppercase();
        let contract_addr = std::env::var(&format!("VDR_ADDRESS_{}", &near_env)).expect(&format!("VDR_ADDRESS_{} must be set.", &near_env));
        
        Self { wallet, contract_address: contract_addr.parse().unwrap() }
    }

    pub fn get_issuer_id(&self) -> String {
        self.wallet.signer_account_id()
    }

    pub async fn get_issuer_schemas(&self) -> Vec<String> {
        let result: Vec<String> = self.wallet.view(
            &self.contract_address,
            "get_issuer_schemas",
            json!({
                "issuer": self.get_issuer_id(),
            })
        ).await.unwrap();

        result
    }

    pub async fn get_schemas(&self, pairs: Vec<(String, u32)>) -> Vec<String> {
        let result: Vec<String> = self.wallet.view(
            &self.contract_address,
            "get_schemas",
            json!({
                "pairs": pairs,
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

    pub async fn check_credentials(&self, pairs: Vec<(String, String)>) -> Vec<bool> {
        let result: Vec<bool> = self.wallet.view(
            &self.contract_address,
            "check_credentials",
            json!({
                "pairs": pairs,
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
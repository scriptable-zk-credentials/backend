use std::{fs, path::Path};
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::{query::QueryResponseKind, chunks::ChunkReference};
use near_primitives::{
    types::{BlockReference, Finality, FunctionArgs, AccountId},
    views::{QueryRequest, FinalExecutionOutcomeView},
    transaction::{Transaction, Action}, hash::CryptoHash,
};
use near_crypto::{InMemorySigner, KeyFile};
use serde_json::{from_slice, Value};


pub struct NearWallet {
    rpc: JsonRpcClient,
    signer: InMemorySigner,
}

impl NearWallet {
    pub fn new() -> Self {
        // Read NEAR account_id and extract the secret key from the wallet file
        let near_env = std::env::var("NEAR_ENV").expect("NEAR_ENV must be set.");
        let rpc_url = format!("https://rpc.{}.near.org", near_env);
        let account_id_key = format!("NEAR_{}_ACCOUNT_ID", near_env).to_uppercase();
        let signer_account_id = std::env::var(&account_id_key).expect(&format!("{} must be set.", &account_id_key));
        let credentials_path = std::env::var("NEAR_CREDENTIALS_PATH").expect("NEAR_CREDENTIALS_PATH must be set.");
        let account_id: AccountId = signer_account_id.parse().unwrap();
        let path_str = format!("{credentials_path}/{near_env}/{signer_account_id}.json");
        let path: &Path = Path::new(&path_str);
        let wallet_file = KeyFile::from_file(path).unwrap();
        
        Self {
            rpc: JsonRpcClient::connect(rpc_url),
            signer: InMemorySigner::from_secret_key(account_id, wallet_file.secret_key)
        }
    }

    pub fn signer_account_id(&self) -> String {
        self.signer.account_id.to_string()
    }

    /// get current nonce and 
    pub async fn get_tx_sign_info(&self) -> Result<(u64, CryptoHash), Box<dyn std::error::Error>> {
        // Query to get nonce
        let access_key_request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::None),
            request: near_primitives::views::QueryRequest::ViewAccessKeyList { account_id: self.signer.account_id.clone() },
        };
        let response = self.rpc.call(&access_key_request).await.unwrap();

        let key_list = match response.kind {
            QueryResponseKind::AccessKeyList(access_key_list) => access_key_list.keys,
            _ => Err("failed to extract info about list of access keys")?,
        };

        let key_info = key_list
            .iter()
            .find(|&info| info.public_key == self.signer.public_key)
            .expect("Used wallet key is not in the active keys list");

        Ok((key_info.access_key.nonce, response.block_hash))
    }

    pub async fn view<T>(&self, address: &AccountId, method: &str, args: Value) -> Result<T, Box<dyn std::error::Error>>
    where T: serde::de::DeserializeOwned
    {
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::None),
            request: QueryRequest::CallFunction {
                account_id: address.parse()?,
                method_name: method.to_string(),
                args: FunctionArgs::from(args.to_string().into_bytes()),
            },
        };

        let response = self.rpc.call(&request).await?;

        match response.kind {
            QueryResponseKind::CallResult(result) => {
                Ok(from_slice::<T>(&result.result)?)
            }
            _ => Err("failed to make view call")?,
        }
    }

    // TODO: turn this into sign_and_send_txs, taking multiple TXs
    pub async fn tx(&self, receiver: &AccountId, actions: Vec<Action>) -> Result<FinalExecutionOutcomeView, Box<dyn std::error::Error>> {
        // get recent block hash and nonce (both can be gotten from the nonce request)
        let (current_nonce, recent_blockhash) = self.get_tx_sign_info().await.unwrap();
        
        // Make & sign the transaction
        let transaction = Transaction {
            signer_id: self.signer.account_id.clone(),
            public_key: self.signer.public_key.clone(),
            nonce: current_nonce + 1,
            receiver_id: receiver.clone(),
            block_hash: recent_blockhash,
            actions,
        };

        let signed_tx = transaction.sign(&self.signer);

        let broadcast_commit_request = methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
            signed_transaction: signed_tx,
        };

        // broadcast request
        let outcome = self.rpc.call(broadcast_commit_request).await.unwrap();
        
        Ok(outcome)
    }

}

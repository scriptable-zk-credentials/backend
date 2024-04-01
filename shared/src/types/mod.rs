use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ScriptLang {
    Rhai,
    JavaScript,
}

pub type SchemaId = u32;

#[derive(Serialize, Deserialize)]
pub struct CredentialInstanceData {
    pub details: String,
    // base64 encoded random integer (u128)
    // why? shorter representation => faster parsing inside zkVM
    pub nonce: String,
    pub schema_id: SchemaId,
}

#[derive(Serialize, Deserialize)]
pub struct ZkvmInput {
    pub credentials: Vec<String>,
    pub lang: ScriptLang,
    pub script: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ZkCommit {
    pub has_error: bool,
    pub err_msg: String,
    pub cred_hashes: Vec<String>,
    pub cred_schemas: Vec<SchemaId>,
    pub lang: ScriptLang,
    pub script: String,
    pub result: bool,
}

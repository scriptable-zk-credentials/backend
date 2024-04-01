#![no_main]


use risc0_zkvm::{
    guest::env,
    sha::{self, Sha256},
};
use shared::types::{ZkCommit, ZkvmInput, CredentialInstanceData, ScriptLang, SchemaId};
use rhai::{Engine, Scope, Dynamic};
//use boa_engine::{Context, Source};
use serde_json::{Value, de::from_str};
use base64ct::{Base64, Encoding};

risc0_zkvm::guest::entry!(main);

pub fn main() {
    let inputs: ZkvmInput = env::read();
    // get stringified credentials
    let credentials_str: Vec<String> = inputs.credentials;

    // get script language
    let script_lang: ScriptLang = inputs.lang;
    // get script
    let input_script: String = inputs.script;

    // validate that credentials are JSON objects with correct structure
    let credentials_res: Result<Vec<CredentialInstanceData>, _> = credentials_str
        .iter()
        .map(|cred_str| from_str(&cred_str))
        .collect();

    // stop if we found any errors
    if credentials_res.is_err() {
        env::commit(&ZkCommit {
            has_error: true,
            err_msg: "failed to parse credentials".to_string(),
            cred_hashes: Vec::new(),
            cred_schemas: Vec::new(),
            lang: inputs.lang,
            script: input_script,
            result: false,
        });
        
        return;
    }

    // safe to unwrap since we check earlier
    let credentials = credentials_res.unwrap();
    let cred_schemas = credentials.iter().map(|data| data.schema_id).collect();

    // calculate sha256 hash of each credential
    let cred_hashes: Vec<String> = credentials_str
        .iter()
        .map(|cred| Base64::encode_string(sha::Impl::hash_bytes(cred.as_bytes()).as_bytes()))
        .collect();

    
    match script_lang {
        ScriptLang::Rhai => {
            let engine = Engine::new_raw();
            let mut scope = Scope::new();
            
            // inject credentials in the script
            let rhai_creds: Dynamic = credentials
                .iter()
                .map(|cred_data| engine.parse_json(cred_data.details.clone(), true).unwrap())
                .collect::<Vec<_>>()
                .into();
            scope.push_constant_dynamic("credentials", rhai_creds);
            
            // run the script
            let raw_result = engine.eval_with_scope::<bool>(&mut scope, &input_script);

            if raw_result.is_err() {
                env::commit(&ZkCommit {
                    has_error: true,
                    err_msg: format!("script error: {}", raw_result.err().unwrap()),
                    cred_hashes,
                    cred_schemas,
                    lang: inputs.lang,
                    script: input_script,
                    result: false,
                });
                
                return;
            }

            env::commit(&ZkCommit {
                has_error: false,
                err_msg: "".to_string(),
                cred_hashes,
                cred_schemas,
                lang: inputs.lang,
                // IMPORTANT!! use input script here to not expose credentials
                script: input_script,
                result: raw_result.unwrap(),
            });
        },
        ScriptLang::JavaScript => (),
    }
}

mod api;
mod db;
mod adapters;

use api::api_start;
use db::db_start;
use dotenv::dotenv;
use adapters::RegistryContract;
use rustpython_vm::{Interpreter, compiler};


#[tokio::main]
async fn main() {
    // load ENV variables
    dotenv().ok();
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    env_logger::init();

    Interpreter::without_stdlib(Default::default()).enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        let source = r#"return 1 == 1"#;
        let code_obj = vm
            .compile(source, compiler::Mode::Exec, "<embedded>".to_owned())
            .map_err(|err| vm.new_syntax_error(&err, Some(source))).unwrap();

        let result = vm.run_code_obj(code_obj, scope).unwrap();
        println!("py result: {:?}", result);
    });

    // adapter to interact with the registry contract
    let registry = RegistryContract::new();
    // start DB
    let db_conn = db_start().await;
    // start API
    api_start(db_conn, registry).await;
}

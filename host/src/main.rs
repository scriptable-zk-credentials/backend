mod api;
mod db;
mod adapters;

use api::api_start;
use db::db_start;
use dotenv::dotenv;
use adapters::RegistryContract;


#[tokio::main]
async fn main() {
    // load ENV variables
    dotenv().ok();
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    env_logger::init();

    // adapter to interact with the registry contract
    let registry = RegistryContract::new();
    // start DB
    let db_conn = db_start().await;
    // start API
    api_start(db_conn, registry).await;
}

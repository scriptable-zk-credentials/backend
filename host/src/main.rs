mod api;
mod db;

use api::api_start;
use db::db_start;
use dotenv::dotenv;


#[tokio::main]
async fn main() {
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    env_logger::init();

    // load ENV variables
    dotenv().ok();

    // start DB
    let db_conn = db_start().await;
    // start API
    api_start(db_conn).await;
}

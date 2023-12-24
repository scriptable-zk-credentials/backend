mod genproof;
mod genscript;

use axum::routing::{Router, post};
use genproof::genproof_handler;
use genscript::genscript_handler;


pub fn holder_router() -> Router {
    Router::new()
        .route("/genproof", post(genproof_handler))
        .route("/genscript", post(genscript_handler))
}

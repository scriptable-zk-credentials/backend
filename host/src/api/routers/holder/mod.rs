mod proof;
mod genscript;

use axum::routing::{Router, post};
use proof::proof_router;
use genscript::genscript_handler;


pub fn holder_router() -> Router {
    Router::new()
        .nest("/proof", proof_router())
        .route("/genscript", post(genscript_handler))
}

mod routers;

use std::sync::Arc;
use axum::{Router, http::Method};
use tower_http::cors::{CorsLayer, Any};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use routers::{
    hello::hello_router,
    holder::holder_router,
    issuer::issuer_router,
    verifier::verifier_router,
};
use sea_orm::DbConn;

use crate::adapters::RegistryContract;


pub async fn api_start(db_connection: DbConn, registry: RegistryContract) {
    let clonabe_registry = Arc::new(registry);
    let api_routes = Router::new()
        .nest("/hello", hello_router())
        .nest("/holder", holder_router())
        .nest("/issuer", issuer_router(db_connection.clone(), Arc::clone(&clonabe_registry)))
        .nest("/verifier", verifier_router(db_connection.clone(), Arc::clone(&clonabe_registry)));

    let addr = SocketAddr::from((
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        3000
    ));
    println!("listening on {}", addr);
    
    let cors = CorsLayer::new()
        .allow_methods(vec![Method::POST, Method::GET])
        .allow_origin(Any)
        .allow_headers(Any);
    let app = Router::new()
        .nest("/", api_routes)
        .layer(cors);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

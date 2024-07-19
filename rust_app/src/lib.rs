pub mod auth;
pub mod config;
pub mod error;
pub mod routes;
pub mod db;

use axum::{Router, Extension, routing::get};
use lambda_http::{run, tracing, Error};
// use tracing_subscriber::field::MakeOutput;
use crate::routes::{root, foo, parameters, health};
use crate::config::Config;
use crate::db::DynamoDb;
// use crate::auth::{auth_middleware, AuthState};
// use std::sync::Arc;

pub async fn create_app(config: Config) -> Router {
    // let auth_state = Arc::new(AuthState::new(config));

    println!("{}",&config.dynamodb_table_name);

    let db = DynamoDb::new(config.dynamodb_table_name)
        .await
        .expect("Failed to initialize DynamoDB client");
    

    Router::new()
        .route("/", get(root::handler))
        .route("/foo", get(foo::get).post(foo::post))
        .route("/foo/:id", get(foo::get_by_id).post(foo::update).delete(foo::delete)) 
        .route("/parameters", get(parameters::handler))
        .route("/health", get(health::check))
        .layer(Extension(db))
        // .layer(Extension(auth_state.clone()))
        // .layer(from_fn(auth_middleware))
}

pub async fn run_app(config: Config) -> Result<(), Error> {
    tracing::init_default_subscriber();
    let app = create_app(config).await;
    run(app).await
}

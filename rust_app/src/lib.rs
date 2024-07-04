pub mod config;
pub mod error;
pub mod routes;

use axum::{Router, routing::get, routing::post};
use lambda_http::{run, tracing, Error};
use crate::routes::{root, foo, parameters, health};
use crate::config::Config;

pub async fn create_app(_config: &Config) -> Router {
    Router::new()
        .route("/", get(root::handler))
        .route("/foo", get(foo::get).post(foo::post))
        .route("/foo/:name", post(foo::post_with_name))
        .route("/parameters", get(parameters::handler))
        .route("/health", get(health::check))
}

pub async fn run_app(config: Config) -> Result<(), Error> {
    tracing::init_default_subscriber();
    let app = create_app(&config).await;
    run(app).await
}


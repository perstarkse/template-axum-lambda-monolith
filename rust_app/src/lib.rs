pub mod auth;
pub mod config;
pub mod error;
pub mod routes;

use axum::{Router, routing::get, routing::post, middleware::from_fn, Extension};
use lambda_http::{run, tracing, Error};
use crate::routes::{root, foo, parameters, health};
use crate::config::Config;
use crate::auth::{auth_middleware, AuthState};
use std::sync::Arc;

pub async fn create_app(config: Config) -> Router {
    let auth_state = Arc::new(AuthState::new(config));

    Router::new()
        .route("/", get(root::handler))
        .route("/foo", get(foo::get).post(foo::post))
        .route("/foo/:name", post(foo::post_with_name))
        .route("/parameters", get(parameters::handler))
        .route("/health", get(health::check))
        .layer(Extension(auth_state.clone()))
        .layer(from_fn(auth_middleware))
}

pub async fn run_app(config: Config) -> Result<(), Error> {
    tracing::init_default_subscriber();
    let app = create_app(config).await;
    run(app).await
}

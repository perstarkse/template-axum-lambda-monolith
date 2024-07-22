pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod middleware;

use auth::Auth;
use axum::middleware::from_fn_with_state;
use axum::{routing::get, Extension, Router};
use lambda_http::{run, tracing, Error};
use middleware::auth_middleware;
// use tracing_subscriber::field::MakeOutput;
use crate::config::Config;
use crate::db::DynamoDb;
use crate::routes::{foo, health, parameters, root};

pub async fn create_app(config: Config) -> Router {
    // println!("{}{}", &config.cognito_user_pool_id, &config.cognito_client_id);

    let auth = Auth::new(
            &config.cognito_region,
            &config.cognito_user_pool_id,
            &config.cognito_client_id
    ).expect("Failed to create Auth");

    // println!("{}", &config.dynamodb_table_name);

    let db = DynamoDb::new(config.dynamodb_table_name)
        .await
        .expect("Failed to initialize DynamoDB client");

    Router::new()
        .route("/", get(root::handler))
        .route("/foo", get(foo::get).post(foo::post))
        .route(
            "/foo/:id",
            get(foo::get_by_id).post(foo::update).delete(foo::delete),
        )
        .route("/parameters", get(parameters::handler))
        .route("/health", get(health::health).layer(from_fn_with_state(auth.clone(), auth_middleware)))
        .layer(Extension(db))
        .layer(Extension(auth))
    // .layer(Extension(auth_state.clone()))
}

pub async fn run_app(config: Config) -> Result<(), Error> {
    tracing::init_default_subscriber();
    let app = create_app(config).await;
    run(app).await
}

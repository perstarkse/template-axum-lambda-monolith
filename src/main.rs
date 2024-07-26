use axum::{middleware::from_fn_with_state, routing::get, Extension, Router};
use lambda_http::{run, Error};

use template::{
    auth::Auth,
    config::Config,
    db::DynamoDbRepository,
    logging,
    middleware::auth_middleware,
    models::item::Item,
    routes::{foo, parameters},
};

async fn create_app(config: Config) -> Router {
    let auth = Auth::new(
        &config.cognito_region,
        &config.cognito_user_pool_id,
        &config.cognito_client_id,
    )
    .expect("Failed to create Auth");

    let db = DynamoDbRepository::<Item>::new(config.dynamodb_table_name)
        .await
        .expect("Failed to initialize DynamoDB client");

    Router::new()
        .route("/parameters", get(parameters::handler))
        .route("/foo", get(foo::get).post(foo::create))
        .route(
            "/foo/:id",
            get(foo::get_by_id).post(foo::update).delete(foo::delete),
        )
        .layer(from_fn_with_state(auth, auth_middleware))
        .layer(Extension(db))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::init_logger();
    let config = Config::from_env();
    let app = create_app(config).await;
    run(app).await
}

use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, patch},
    Extension, Router,
};
use lambda_http::{run, Error};

use template::{
    auth::secret_auth_middleware::{secret_middleware, SecretAuth},
    config::{AuthMethod, Config},
    db::DynamoDbRepository,
    logging,
    models::{item::Item, user::User},
    routes::{foo, parameters, user},
};

async fn create_app(config: Config) -> Router {
    match config.auth_method {
        AuthMethod::Cognito => {
            panic!("We are using the secret method for this api");
        }
        AuthMethod::Secret => {
            let auth = SecretAuth::new(config.secret.unwrap());

            let user_db = DynamoDbRepository::<User>::new(config.dynamodb_user_table_name.unwrap())
                .await
                .expect("Failed to initialize DynamoDB client for user table");

            let db = DynamoDbRepository::<Item>::new(config.dynamodb_table_name)
                .await
                .expect("Failed to initialize DynamoDB client for item table");

            Router::new()
                .route("/parameters", get(parameters::handler))
                .route("/foo", get(foo::get).post(foo::create))
                .route("/user", get(user::get))
                .route("/user/:id", delete(user::delete))
                .route(
                    "/foo/:id",
                    get(foo::get_by_id).post(foo::update).delete(foo::delete),
                )
                .route("/user/:id/admin-status", patch(user::patch_admin_status))
                .route_layer(from_fn_with_state(auth.clone(), secret_middleware))
                .layer(Extension(db))
                .layer(Extension(user_db))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::init_logger();
    let config = Config::from_env();
    let app = create_app(config).await;
    run(app).await
}

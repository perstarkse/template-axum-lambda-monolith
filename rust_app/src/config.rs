use std::env;

pub struct Config {
    pub environment: String,
    pub cognito_user_pool_id: String,
    pub cognito_app_client_id: String,
    pub aws_region: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            cognito_user_pool_id: env::var("COGNITO_USER_POOL_ID")
                .expect("COGNITO_USER_POOL_ID must be set"),
            cognito_app_client_id: env::var("COGNITO_APP_CLIENT_ID")
                .expect("COGNITO_APP_CLIENT_ID must be set"),
            aws_region: env::var("AWS_REGION").expect("AWS_REGION must be set"),
        }
    }
}

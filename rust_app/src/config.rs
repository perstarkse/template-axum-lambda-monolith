use std::env;

pub struct Config {
    pub environment: String,
    pub dynamodb_table_name: String,
    pub aws_region: String,
    // pub cognito_user_pool_id: String,
    // pub cognito_app_client_id: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            aws_region: env::var("AWS_REGION").expect("AWS_REGION must be set"),
            dynamodb_table_name: env::var("TEST_TABLE_NAME").expect("TEST_TABLE_NAME must be set"),
            // cognito_user_pool_id: env::var("COGNITO_USER_POOL_ID")
            // .expect("COGNITO_USER_POOL_ID must be set"),
            // cognito_app_client_id: env::var("COGNITO_APP_CLIENT_ID")
            // .expect("COGNITO_APP_CLIENT_ID must be set"),
        }
    }
}

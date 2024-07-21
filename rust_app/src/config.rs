use std::env;

pub struct Config {
    pub dynamodb_table_name: String,
    pub aws_region: String,
    pub cognito_user_pool_id: String,
    pub cognito_client_id: String,
    pub cognito_region: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            aws_region: env::var("AWS_REGION").expect("AWS_REGION must be set"),
            dynamodb_table_name: env::var("TEST_TABLE_NAME").expect("TEST_TABLE_NAME must be set"),
            cognito_region: env::var("COGNITO_REGION").expect("COGNITO_REGION must be set"),
            cognito_user_pool_id: env::var("COGNITO_USER_POOL_ID")
                .expect("COGNITO_USER_POOL_ID must be set"),
            cognito_client_id: env::var("COGNITO_CLIENT_ID")
                .expect("COGNITO_CLIENT_ID must be set"),
        }
    }
}

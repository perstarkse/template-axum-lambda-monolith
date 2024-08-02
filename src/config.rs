use std::env;

pub enum AuthMethod {
    Cognito,
    Secret,
}

pub struct Config {
    pub aws_region: String,
    pub dynamodb_table_name: String,
    pub dynamodb_user_table_name: Option<String>,
    pub auth_method: AuthMethod,
    pub cognito_region: Option<String>,
    pub cognito_user_pool_id: Option<String>,
    pub cognito_client_id: Option<String>,
    pub secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let auth_method = env::var("AUTH_METHOD").expect("AUTH_METHOD must be set");
        let auth_method = match auth_method.as_str() {
            "COGNITO" => AuthMethod::Cognito,
            "SECRET" => AuthMethod::Secret,
            _ => panic!("Invalid AUTH_METHOD"),
        };

        match auth_method {
            AuthMethod::Cognito => Config {
                aws_region: env::var("AWS_REGION").expect("AWS_REGION must be set"),
                dynamodb_table_name: env::var("TEST_TABLE_NAME")
                    .expect("TEST_TABLE_NAME must be set"),
                dynamodb_user_table_name: None,
                auth_method,
                cognito_region: Some(
                    env::var("COGNITO_REGION").expect("COGNITO_REGION must be set"),
                ),
                cognito_user_pool_id: Some(
                    env::var("COGNITO_USER_POOL_ID").expect("COGNITO_USER_POOL_ID must be set"),
                ),
                cognito_client_id: Some(
                    env::var("COGNITO_CLIENT_ID").expect("COGNITO_CLIENT_ID must be set"),
                ),
                secret: None,
            },
            AuthMethod::Secret => Config {
                aws_region: env::var("AWS_REGION").expect("AWS_REGION must be set"),
                dynamodb_table_name: env::var("TEST_TABLE_NAME")
                    .expect("TEST_TABLE_NAME must be set"),
                dynamodb_user_table_name: Some(
                    env::var("USER_TABLE_NAME").expect("USER_TABLE_NAME must be set"),
                ),
                auth_method,
                cognito_region: None,
                cognito_user_pool_id: None,
                cognito_client_id: None,
                secret: Some(env::var("SECRET").expect("SECRET must be set")),
            },
        }
    }
}

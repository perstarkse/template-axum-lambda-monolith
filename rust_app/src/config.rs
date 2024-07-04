use std::env;

pub struct Config {
    pub environment: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }
}

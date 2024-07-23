use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug)]
pub enum AppError {
    EnvError(std::env::VarError),
    // Add more error types as needed
}

impl From<std::env::VarError> for AppError {
    fn from(err: std::env::VarError) -> Self {
        AppError::EnvError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::EnvError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            // Handle other error types
        };
        (status, error_message).into_response()
    }
}

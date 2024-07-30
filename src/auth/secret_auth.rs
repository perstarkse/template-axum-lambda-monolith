use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;

#[derive(Clone)]
pub struct SecretAuth {
    pub secret: String,
}

impl SecretAuth {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

pub async fn secret_middleware(
    State(state): State<SecretAuth>,
    req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) if token == state.secret => next.run(req).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

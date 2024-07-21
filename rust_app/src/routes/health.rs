use axum::{
    extract::Request, http::StatusCode, Extension, Json
};
use serde_json::{json, Value};

use crate::auth::Auth;

pub async fn health(
    Extension(auth): Extension<Auth>,
    request: Request,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let auth_header = request.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            match auth.verify_token(token).await {
                Ok(_) => Ok(Json(json!({"status": "Healthy and Authenticated"}))),
                Err(e) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": format!("Invalid token: {:?}", e)}))
                )),
            }
        },
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "No token provided"}))
        )),
    }
}

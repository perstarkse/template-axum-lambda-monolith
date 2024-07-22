use axum::{Extension, Json};
use serde_json::{json, Value};
use crate::auth::Claims;

pub async fn health(
    Extension(claims): Extension<Claims>,
) -> Json<Value> {
    Json(json!({
        "status": "Healthy and Authenticated",
        "user": claims.username,
    }))
}

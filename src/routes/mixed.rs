use axum::{extract::Extension, Json};
use serde_json::{json, Value};
use crate::auth::Claims;

pub async fn mixed_handler(
    claims: Option<Extension<Claims>>,
) -> Json<Value> {
    match claims {
        Some(claims) => Json(json!({
            "message": "You are authenticated",
            "user": claims.username,
            "additional_info": "Here's some extra information for authenticated users"
        })),
        None => Json(json!({
            "message": "You are not authenticated",
            "public_info": "This is publicly available information"
        })),
    }
}


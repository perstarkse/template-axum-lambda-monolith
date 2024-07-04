use axum::{Json, extract::Path};
use serde_json::{json, Value};

pub async fn get() -> Json<Value> {
    Json(json!({ "msg": "I am GET /foo" }))
}

pub async fn post() -> Json<Value> {
    Json(json!({ "msg": "I am POST /foo" }))
}

pub async fn post_with_name(Path(name): Path<String>) -> Json<Value> {
    Json(json!({ "msg": format!("I am POST /foo/:name, name={name}") }))
}


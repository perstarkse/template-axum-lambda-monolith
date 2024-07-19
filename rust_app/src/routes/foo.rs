use axum::{Json, extract::Path};
use serde_json::{json, Value};
use axum::Extension;
use crate::db::{DynamoDb, DynamoDbTrait, CreateItem};

pub async fn get() -> Json<Value> {
    Json(json!({ "message": "I am GET /foo" }))
}

pub async fn post(Extension(db): Extension<DynamoDb>, Json(create_item): Json<CreateItem>) -> Json<Value> {
    match db.create(create_item).await {
        Ok(id) => Json(json!({ "message": "success", "id": id })),
        Err(e) => Json(json!({ "message": "failure", "error": e.to_string() })),
    }
}

pub async fn post_with_name(Path(name): Path<String>) -> Json<Value> {
    Json(json!({ "message": format!("I am POST /foo/:name, name={name}") }))
}


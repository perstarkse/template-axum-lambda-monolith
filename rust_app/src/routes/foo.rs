use axum::{Json, extract::Path};
use serde_json::{json, Value};
use axum::Extension;
use crate::db::{DynamoDb, Item};

pub async fn get() -> Json<Value> {
    Json(json!({ "message": "I am GET /foo" }))
}

// pub async fn post() -> Json<Value> {
    
    // Json(json!({ "message": "I am POST /foo" }))
// }
pub async fn post(Extension(db): Extension<DynamoDb>, Json(item): Json<Item>) -> Json<Value> {
    match db.put_item(item).await {
        Ok(_) => Json(json!({ "message": "success" })),
        Err(e) => Json(json!({ "message": "failure", "error": e.to_string() })),
    }
}
// pub async fn post(Extension(db): Extension<DynamoDb>,Json(item): Json<Item>) -> StatusCode {
//     match db.put_item(item).await {
//         Ok(_) => StatusCode::CREATED,
//         Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
//     }
// }

pub async fn post_with_name(Path(name): Path<String>) -> Json<Value> {
    Json(json!({ "message": format!("I am POST /foo/:name, name={name}") }))
}


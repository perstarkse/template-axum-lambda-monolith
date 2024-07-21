use crate::db::{CreateItem, DynamoDb, DynamoDbTrait, Item};
use axum::Extension;
use axum::{extract::Path, Json};
use serde_json::{json, Value};

pub async fn get(Extension(db): Extension<DynamoDb>) -> Json<Value> {
    match db.scan().await {
        Ok(items) => Json(json!({ "items": items })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn get_by_id(Extension(db): Extension<DynamoDb>, Path(id): Path<String>) -> Json<Value> {
    match db.get_item(&id).await {
        Ok(Some(item)) => Json(json!(item)),
        Ok(None) => Json(json!({ "error": "Item not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn post(
    Extension(db): Extension<DynamoDb>,
    Json(create_item): Json<CreateItem>,
) -> Json<Value> {
    match db.create(create_item).await {
        Ok(id) => Json(json!({ "message": "success", "id": id })),
        Err(e) => Json(json!({ "message": "failure", "error": e.to_string() })),
    }
}
pub async fn update(
    Extension(db): Extension<DynamoDb>,
    Path(id): Path<String>,
    Json(item): Json<Item>,
) -> Json<Value> {
    if id != item.id {
        return Json(json!({ "error": "ID in path does not match ID in item" }));
    }

    match db.update(item).await {
        Ok(()) => Json(json!({ "message": "Item updated successfully" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn delete(Extension(db): Extension<DynamoDb>, Path(id): Path<String>) -> Json<Value> {
    match db.delete(&id).await {
        Ok(()) => Json(json!({ "message": "Item deleted successfully" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

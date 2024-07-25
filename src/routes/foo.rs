use crate::auth::Claims;
use crate::db::{DynamoDbOperations, DynamoDbRepository};
use crate::models::item::{CreateItem, Item};
use axum::Extension;
use axum::{extract::Path, Json};
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn get(Extension(db): Extension<DynamoDbRepository<Item>>) -> Json<Value> {
    match db.scan().await {
        Ok(items) => Json(json!({ "items": items })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn get_by_id(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Path(id): Path<String>,
) -> Json<Value> {
    match db.get_item(&id).await {
        Ok(Some(item)) => Json(json!(item)),
        Ok(None) => Json(json!({ "error": "Item not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn post(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Json(create_item): Json<CreateItem>,
) -> Json<Value> {
    let item = Item {
        id: Uuid::new_v4().to_string(),
        name: create_item.name,
        age: create_item.age,
        deleted_at: None,
        deleted_by: None,
    };

    match db.create(item).await {
        Ok(id) => Json(json!({ "message": "success", "id": id })),
        Err(e) => Json(json!({ "message": "failure", "error": e.to_string() })),
    }
}

pub async fn update(
    Extension(db): Extension<DynamoDbRepository<Item>>,
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

pub async fn delete(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Path(id): Path<String>,
    claims: Option<Extension<Claims>>,
) -> Json<Value> {
    match claims {
        Some(claims) => match db.soft_delete(&id, &claims.username).await {
            Ok(()) => Json(json!({ "message": "Item deleted successfully" })),
            Err(e) => Json(json!({ "error": e.to_string() })),
        },
        None => Json(json!({
            "message": "You are not authenticated",
            "public_info": "This is publicly available information"
        })),
    }
}

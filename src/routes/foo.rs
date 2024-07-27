use crate::auth::Claims;
use crate::db::{DynamoDbOperations, DynamoDbRepository, OperationResult};
use crate::models::item::{CreateItem, Item};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum::{extract::Path, Json};
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

pub async fn get(Extension(db): Extension<DynamoDbRepository<Item>>) -> Response {
    match db.scan().await {
        OperationResult::Success(data) => {
            (StatusCode::OK, Json(json!({"items": data}))).into_response()
        }
        err => err.into_response(),
    }
}

pub async fn get_by_id(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Path(id): Path<String>,
) -> Response {
    match db.get_item(id).await {
        OperationResult::Success(item) => {
            (StatusCode::OK, Json(json!({"item": item}))).into_response()
        }
        err => err.into_response(),
    }
}

pub async fn create(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Json(create_item): Json<CreateItem>,
) -> Response {
    let item = Item {
        id: Uuid::new_v4().to_string(),
        name: create_item.name,
        age: create_item.age,
        deleted_at: None,
        deleted_by: None,
    };
    let item_id = item.id.clone();

    match db.create(item).await {
        OperationResult::Success(_) => (
            StatusCode::CREATED,
            Json(json!({
                "message": "Item was successfully created",
                "item_id": item_id
            })),
        )
            .into_response(),
        err => err.into_response(),
    }
}

pub async fn update(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Path(id): Path<String>,
    Json(item): Json<Item>,
) -> Response {
    if id != item.id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "ID in path does not match ID in item" })),
        )
            .into_response();
    }

    match db.update(item).await {
        OperationResult::Success(_) => (
            StatusCode::OK,
            Json(json!({
                "message": "Item was successfully updated",
            })),
        )
            .into_response(),
        err => err.into_response(),
    }
}

pub async fn delete(
    Extension(db): Extension<DynamoDbRepository<Item>>,
    Path(id): Path<String>,
    claims: Option<Extension<Claims>>,
) -> Response {
    match claims {
        Some(claims) => match db.soft_delete(id, claims.username.clone()).await {
            OperationResult::Success(_) => (
                StatusCode::OK,
                Json(json!({"message": "Item was successfully removed"})),
            )
                .into_response(),
            err => err.into_response(),
        },
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "message": "You are not authenticated",
            })),
        )
            .into_response(),
    }
}

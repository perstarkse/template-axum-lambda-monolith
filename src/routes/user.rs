use crate::db::{DynamoDbOperations, DynamoDbRepository, OperationResult};
use crate::models::user::{User, UserDynamoDbRepository};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum::{extract::Path, Json};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub async fn get(Extension(db): Extension<DynamoDbRepository<User>>) -> Response {
    match db.scan().await {
        OperationResult::Success(data) => (StatusCode::OK, Json(json!(data))).into_response(),
        err => err.into_response(),
    }
}

pub async fn delete(
    Extension(db): Extension<DynamoDbRepository<User>>,
    Path(id): Path<String>,
) -> Response {
    match db.soft_delete(id, "admin".to_string()).await {
        OperationResult::Success(_) => (
            StatusCode::NO_CONTENT,
            Json(json!({"message": "Item was successfully removed"})),
        )
            .into_response(),
        err => err.into_response(),
    }
}
pub async fn patch_admin_status(
    Extension(db): Extension<DynamoDbRepository<User>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAdminStatusRequest>,
) -> Response {
    match UserDynamoDbRepository::update_admin_status(db, id, body.admin).await {
        OperationResult::Success(_) => (
            StatusCode::OK,
            Json(json!({"message": "Admin status was successfully updated"})),
        )
            .into_response(),
        err => err.into_response(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAdminStatusRequest {
    pub admin: bool,
}

use aws_sdk_dynamodb::{operation::update_item::UpdateItemError, types::AttributeValue};
use axum::async_trait;
use serde::{Deserialize, Serialize};

use crate::db::{DynamoDbOperations, DynamoDbRepository, OperationResult, SoftDeletable};

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub created_at: String,
    pub email_verified: bool,
    pub password_hash: Option<String>,
    pub admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_by: Option<String>,
}

#[async_trait]
impl SoftDeletable for User {
    fn get_deleted_at(&self) -> &Option<String> {
        &self.deleted_at
    }
}

#[async_trait]
pub trait UserDynamoDbRepository: DynamoDbOperations<User> {
    async fn update_admin_status(self, id: String, admin: bool) -> OperationResult<User>;
}

#[async_trait]
impl UserDynamoDbRepository for DynamoDbRepository<User> {
    async fn update_admin_status(self, id: String, admin: bool) -> OperationResult<User> {
        let admin_value = AttributeValue::Bool(admin);

        match self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .update_expression("SET admin = :admin")
            .expression_attribute_values(":admin", admin_value)
            .condition_expression("attribute_exists(id) AND attribute_not_exists(deleted_at)")
            .send()
            .await
        {
            Ok(_) => OperationResult::Success(None),
            Err(err) => match err.into_service_error() {
                UpdateItemError::ConditionalCheckFailedException(_) => {
                    OperationResult::ItemNotFound
                }
                _ => OperationResult::InternalError("Service Error".to_string()),
            },
        }
    }
}

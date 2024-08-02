use axum::async_trait;
use serde::{Deserialize, Serialize};

use crate::db::SoftDeletable;

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub created_at: String,
    pub email_verified: bool,
    pub password_hash: Option<String>,
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

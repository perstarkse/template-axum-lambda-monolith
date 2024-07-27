use axum::async_trait;
use serde::{Deserialize, Serialize};

use crate::db::SoftDeletable;

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub age: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateItem {
    pub name: String,
    pub age: u32,
}

#[async_trait]
impl SoftDeletable for Item {
    fn get_deleted_at(&self) -> &Option<String> {
        &self.deleted_at
    }
}

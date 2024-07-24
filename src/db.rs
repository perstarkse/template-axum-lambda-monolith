use anyhow::Result;
use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client, Error};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, to_item};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[cfg_attr(test, automock)]
#[async_trait]
pub trait DynamoDbTrait {
    async fn get_item(&self, id: &str) -> Result<Option<Item>>;
    async fn create(&self, item: CreateItem) -> Result<String>;
    async fn update(&self, item: Item) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn soft_delete(&self, id: &str, user_id: &str) -> Result<()>;
    async fn scan(&self) -> Result<Vec<Item>>;
    async fn get_deleted_items_by_user(&self, user_id: &str) -> Result<Vec<Item>>;
    async fn get_deleted_items(&self) -> Result<Vec<Item>>;
}

#[derive(Clone)]
pub struct DynamoDb {
    pub client: Client,
    pub table_name: String,
}

impl DynamoDb {
    pub async fn new(table_name: String) -> Result<Self, Error> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);

        Ok(Self { client, table_name })
    }
}

#[async_trait]
impl DynamoDbTrait for DynamoDb {
    async fn get_item(&self, id: &str) -> Result<Option<Item>> {
        let key = HashMap::from([("id".to_string(), AttributeValue::S(id.to_string()))]);

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .send()
            .await?;

        if let Some(item) = result.item {
            let item: Item = from_item(item)?;
            if item.deleted_at.is_none() {
                Ok(Some(item))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn scan(&self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let scan_output = self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("attribute_not_exists(deleted_at)")
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await?;

            if let Some(scanned_items) = scan_output.items {
                for item in scanned_items {
                    items.push(from_item(item)?);
                }
            }

            last_evaluated_key = scan_output.last_evaluated_key;

            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(items)
    }

    async fn update(&self, item: Item) -> Result<()> {
        let dynamo_item = to_item(item)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_exists(id) AND attribute_not_exists(deleted_at)")
            .send()
            .await?;

        Ok(())
    }

    async fn create(&self, create_item: CreateItem) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let item = Item {
            id: id.clone(),
            name: create_item.name,
            age: create_item.age,
            deleted_at: None,
            deleted_by: None,
        };
        let dynamo_item = to_item(item)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_not_exists(id)")
            .send()
            .await?;

        Ok(id)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let key = HashMap::from([("id".to_string(), AttributeValue::S(id.to_string()))]);

        self.client
            .delete_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .send()
            .await?;

        Ok(())
    }

    async fn soft_delete(&self, id: &str, user_id: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            .to_string();

        let update_expression = "SET deleted_at = :deleted_at, deleted_by = :deleted_by";

        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .update_expression(update_expression)
            .expression_attribute_values(":deleted_at", AttributeValue::S(now))
            .expression_attribute_values(":deleted_by", AttributeValue::S(user_id.to_string()))
            .send()
            .await?;

        Ok(())
    }

    async fn get_deleted_items_by_user(&self, user_id: &str) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let scan_output = self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("deleted_by = :user_id")
                .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await?;

            if let Some(scanned_items) = scan_output.items {
                for item in scanned_items {
                    items.push(from_item(item)?);
                }
            }

            last_evaluated_key = scan_output.last_evaluated_key;

            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(items)
    }

    async fn get_deleted_items(&self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let scan_output = self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("attribute_exists(deleted_at)")
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await?;

            if let Some(scanned_items) = scan_output.items {
                for item in scanned_items {
                    items.push(from_item(item)?);
                }
            }

            last_evaluated_key = scan_output.last_evaluated_key;

            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_scan() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_scan().times(1).returning(|| {
            Ok(vec![
                Item {
                    id: "id1".to_string(),
                    name: "Name 1".to_string(),
                    age: 30,
                    deleted_at: None,
                    deleted_by: None,
                },
                Item {
                    id: "id2".to_string(),
                    name: "Name 2".to_string(),
                    age: 40,
                    deleted_at: None,
                    deleted_by: None,
                },
            ])
        });

        let result = mock.scan().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "id1");
        assert_eq!(result[1].id, "id2");
    }

    #[tokio::test]
    async fn test_get_item() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_get_item()
            .with(eq("test_id"))
            .times(1)
            .returning(|_| {
                Ok(Some(Item {
                    id: "test_id".to_string(),
                    name: "Test Name".to_string(),
                    age: 30,
                    deleted_at: None,
                    deleted_by: None,
                }))
            });

        let result = mock.get_item("test_id").await.unwrap();
        assert!(result.is_some());
        let item = result.unwrap();
        assert_eq!(item.id, "test_id");
        assert_eq!(item.name, "Test Name");
        assert_eq!(item.age, 30);
    }

    #[tokio::test]
    async fn test_get_deleted_item() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_get_item()
            .with(eq("deleted_id"))
            .times(1)
            .returning(|_| Ok(None));

        let result = mock.get_item("deleted_id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_create()
            .with(function(|item: &CreateItem| {
                item.name == "Test Name" && item.age == 30
            }))
            .times(1)
            .returning(|_| Ok("new_id".to_string()));

        let create_item = CreateItem {
            name: "Test Name".to_string(),
            age: 30,
        };

        let result = mock.create(create_item).await.unwrap();
        assert_eq!(result, "new_id");
    }

    #[tokio::test]
    async fn test_update() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_update()
            .with(function(|item: &Item| {
                item.id == "test_id" && item.name == "Updated Name" && item.age == 31
            }))
            .times(1)
            .returning(|_| Ok(()));

        let item = Item {
            id: "test_id".to_string(),
            name: "Updated Name".to_string(),
            age: 31,
            deleted_at: None,
            deleted_by: None,
        };

        let result = mock.update(item).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_delete()
            .with(eq("test_id"))
            .times(1)
            .returning(|_| Ok(()));

        let result = mock.delete("test_id").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_soft_delete()
            .with(eq("test_id"), eq("user1"))
            .times(1)
            .returning(|_, _| Ok(()));

        let result = mock.soft_delete("test_id", "user1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_deleted_items_by_user() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_get_deleted_items_by_user()
            .with(eq("user1"))
            .times(1)
            .returning(|_| {
                Ok(vec![Item {
                    id: "deleted_id".to_string(),
                    name: "Deleted Item".to_string(),
                    age: 25,
                    deleted_at: Some("2023-05-01T12:00:00Z".to_string()),
                    deleted_by: Some("user1".to_string()),
                }])
            });

        let result = mock.get_deleted_items_by_user("user1").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "deleted_id");
        assert_eq!(result[0].deleted_by, Some("user1".to_string()));
    }

    #[tokio::test]
    async fn test_get_deleted_items() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_get_deleted_items().times(1).returning(|| {
            Ok(vec![
                Item {
                    id: "deleted_id1".to_string(),
                    name: "Deleted Item 1".to_string(),
                    age: 25,
                    deleted_at: Some("2023-05-01T12:00:00Z".to_string()),
                    deleted_by: Some("user1".to_string()),
                },
                Item {
                    id: "deleted_id2".to_string(),
                    name: "Deleted Item 2".to_string(),
                    age: 30,
                    deleted_at: Some("2023-05-02T12:00:00Z".to_string()),
                    deleted_by: Some("user2".to_string()),
                },
            ])
        });

        let result = mock.get_deleted_items().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "deleted_id1");
        assert_eq!(result[1].id, "deleted_id2");
    }

    #[tokio::test]
    async fn test_update_deleted_item() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_update()
            .with(function(|item: &Item| {
                item.id == "deleted_id" && item.deleted_at.is_some()
            }))
            .times(1)
            .returning(|_| Err(anyhow::anyhow!("Cannot update deleted item")));

        let item = Item {
            id: "deleted_id".to_string(),
            name: "Deleted Item".to_string(),
            age: 25,
            deleted_at: Some("2023-05-01T12:00:00Z".to_string()),
            deleted_by: Some("user1".to_string()),
        };

        let result = mock.update(item).await;
        assert!(result.is_err());
    }
}

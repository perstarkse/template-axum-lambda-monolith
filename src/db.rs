use anyhow::Result;
use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client, Error};
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, to_item};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[async_trait]
impl SoftDeletable for Item {
    fn get_deleted_at(&self) -> &Option<String> {
        &self.deleted_at
    }
}

#[async_trait]
pub trait SoftDeletable: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync {
    fn get_deleted_at(&self) -> &Option<String>;
}

#[async_trait]
pub trait DynamoDbOperations<T>: Send + Sync {
    async fn get_item(&self, id: &str) -> Result<Option<T>>;
    async fn create(&self, item: T) -> Result<String>;
    async fn update(&self, item: T) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn soft_delete(&self, id: &str, user_id: &str) -> Result<()>;
    async fn scan(&self) -> Result<Vec<T>>;
    async fn get_deleted_items_by_user(&self, user_id: &str) -> Result<Vec<T>>;
    async fn get_deleted_items(&self) -> Result<Vec<T>>;
}

#[derive(Clone)]
pub struct DynamoDbRepository<T> {
    pub client: Client,
    pub table_name: String,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<T> DynamoDbRepository<T> {
    pub async fn new(table_name: String) -> Result<Self, Error> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            table_name,
            _phantom: std::marker::PhantomData,
        })
    }
}

#[async_trait]
impl<T> DynamoDbOperations<T> for DynamoDbRepository<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static + SoftDeletable,
{
    async fn get_item(&self, id: &str) -> Result<Option<T>> {
        let key = HashMap::from([("id".to_string(), AttributeValue::S(id.to_string()))]);

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .send()
            .await?;

        if let Some(item) = result.item {
            let item: T = from_item(item)?;
            if item.get_deleted_at().is_none() {
                Ok(Some(item))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn scan(&self) -> Result<Vec<T>> {
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

    async fn update(&self, item: T) -> Result<()> {
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

    async fn create(&self, item: T) -> Result<String> {
        let dynamo_item = to_item(item)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_not_exists(id)")
            .send()
            .await?;

        Ok("Item successfully created".to_string())
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

        let result = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .update_expression("SET deleted_at = :deleted_at, deleted_by = :deleted_by")
            .condition_expression("attribute_exists(id) AND attribute_not_exists(deleted_at)")
            .expression_attribute_values(":deleted_at", AttributeValue::S(now))
            .expression_attribute_values(":deleted_by", AttributeValue::S(user_id.to_string()))
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                if let aws_sdk_dynamodb::error::SdkError::ServiceError(service_err) = &err {
                    if service_err.err().is_conditional_check_failed_exception() {
                        return Err(anyhow::anyhow!("Item is already deleted or doesn't exist"));
                    }
                }
                Err(anyhow::anyhow!("Failed to soft delete item: {}", err))
            }
        }
    }

    async fn get_deleted_items_by_user(&self, user_id: &str) -> Result<Vec<T>> {
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

    async fn get_deleted_items(&self) -> Result<Vec<T>> {
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
    use mockall::*;
    use tokio;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct TestItem {
        pub id: String,
        pub name: String,
        pub age: u32,
        pub deleted_at: Option<String>,
        pub deleted_by: Option<String>,
    }

    #[async_trait]
    impl SoftDeletable for TestItem {
        fn get_deleted_at(&self) -> &Option<String> {
            &self.deleted_at
        }
    }

    mock! {
        pub DynamoDbTestItem {}

        #[async_trait]
        impl DynamoDbOperations<TestItem> for DynamoDbTestItem {
            async fn get_item(&self, id: &str) -> Result<Option<TestItem>>;
            async fn create(&self, item: TestItem) -> Result<String>;
            async fn update(&self, item: TestItem) -> Result<()>;
            async fn delete(&self, id: &str) -> Result<()>;
            async fn soft_delete(&self, id: &str, user_id: &str) -> Result<()>;
            async fn scan(&self) -> Result<Vec<TestItem>>;
            async fn get_deleted_items_by_user(&self, user_id: &str) -> Result<Vec<TestItem>>;
            async fn get_deleted_items(&self) -> Result<Vec<TestItem>>;
        }
    }

    #[tokio::test]
    async fn test_get_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let test_item = TestItem {
            id: "test_id".to_string(),
            name: "test_name".to_string(),
            age: 30,
            deleted_at: None,
            deleted_by: None,
        };

        mock_db
            .expect_get_item()
            .with(eq("test_id"))
            .returning(move |_| Ok(Some(test_item.clone())));

        let result = mock_db.get_item("test_id").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "test_name");
    }

    #[tokio::test]
    async fn test_get_item_not_found() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_get_item()
            .with(eq("non_existing_id"))
            .returning(|_| Ok(None));

        let result = mock_db.get_item("non_existing_id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let test_item = TestItem {
            id: "new_id".to_string(),
            name: "new_name".to_string(),
            age: 25,
            deleted_at: None,
            deleted_by: None,
        };

        mock_db
            .expect_create()
            .returning(|_| Ok("Item successfully created".to_string()));

        let result = mock_db.create(test_item).await.unwrap();
        assert_eq!(result, "Item successfully created");
    }

    #[tokio::test]
    async fn test_create_item_fail() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let test_item = TestItem {
            id: "existing_id".to_string(),
            name: "existing_name".to_string(),
            age: 40,
            deleted_at: None,
            deleted_by: None,
        };

        mock_db
            .expect_create()
            .returning(|_| Err(anyhow::anyhow!("Failed to create item")));

        let result = mock_db.create(test_item).await;
        assert!(result.is_err());
    }
    #[tokio::test]
    async fn test_update_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let test_item = TestItem {
            id: "update_id".to_string(),
            name: "updated_name".to_string(),
            age: 35,
            deleted_at: None,
            deleted_by: None,
        };

        mock_db.expect_update().returning(|_| Ok(()));

        let result = mock_db.update(test_item).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_delete()
            .with(eq("delete_id"))
            .returning(|_| Ok(()));

        let result = mock_db.delete("delete_id").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_soft_delete()
            .with(eq("soft_delete_id"), eq("user_123"))
            .returning(|_, _| Ok(()));

        let result = mock_db.soft_delete("soft_delete_id", "user_123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scan_items() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let test_items = vec![
            TestItem {
                id: "id1".to_string(),
                name: "name1".to_string(),
                age: 30,
                deleted_at: None,
                deleted_by: None,
            },
            TestItem {
                id: "id2".to_string(),
                name: "name2".to_string(),
                age: 40,
                deleted_at: None,
                deleted_by: None,
            },
        ];

        mock_db
            .expect_scan()
            .returning(move || Ok(test_items.clone()));

        let result = mock_db.scan().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "name1");
        assert_eq!(result[1].name, "name2");
    }

    #[tokio::test]
    async fn test_get_deleted_items_by_user() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let deleted_items = vec![
            TestItem {
                id: "del_id1".to_string(),
                name: "del_name1".to_string(),
                age: 50,
                deleted_at: Some("2023-05-01".to_string()),
                deleted_by: Some("user_456".to_string()),
            },
            TestItem {
                id: "del_id2".to_string(),
                name: "del_name2".to_string(),
                age: 60,
                deleted_at: Some("2023-05-02".to_string()),
                deleted_by: Some("user_456".to_string()),
            },
        ];

        mock_db
            .expect_get_deleted_items_by_user()
            .with(eq("user_456"))
            .returning(move |_| Ok(deleted_items.clone()));

        let result = mock_db.get_deleted_items_by_user("user_456").await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].deleted_by, Some("user_456".to_string()));
        assert_eq!(result[1].deleted_by, Some("user_456".to_string()));
    }
    #[tokio::test]
    async fn test_get_deleted_items() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let deleted_items = vec![
            TestItem {
                id: "del_id1".to_string(),
                name: "del_name1".to_string(),
                age: 50,
                deleted_at: Some("2023-05-01".to_string()),
                deleted_by: Some("user_123".to_string()),
            },
            TestItem {
                id: "del_id2".to_string(),
                name: "del_name2".to_string(),
                age: 60,
                deleted_at: Some("2023-05-02".to_string()),
                deleted_by: Some("user_456".to_string()),
            },
        ];

        mock_db
            .expect_get_deleted_items()
            .returning(move || Ok(deleted_items.clone()));

        let result = mock_db.get_deleted_items().await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].deleted_at.is_some());
        assert!(result[1].deleted_at.is_some());
    }

    #[tokio::test]
    async fn test_update_item_fail() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let test_item = TestItem {
            id: "update_fail_id".to_string(),
            name: "update_fail_name".to_string(),
            age: 35,
            deleted_at: None,
            deleted_by: None,
        };

        mock_db
            .expect_update()
            .returning(|_| Err(anyhow::anyhow!("Failed to update item")));

        let result = mock_db.update(test_item).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_item_fail() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_delete()
            .with(eq("delete_fail_id"))
            .returning(|_| Err(anyhow::anyhow!("Failed to delete item")));

        let result = mock_db.delete("delete_fail_id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_soft_delete_item_fail() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_soft_delete()
            .with(eq("soft_delete_fail_id"), eq("user_789"))
            .returning(|_, _| Err(anyhow::anyhow!("Failed to soft delete item")));

        let result = mock_db.soft_delete("soft_delete_fail_id", "user_789").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scan_items_empty() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db.expect_scan().returning(move || Ok(vec![]));

        let result = mock_db.scan().await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_deleted_items_by_user_empty() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_get_deleted_items_by_user()
            .with(eq("non_deleting_user"))
            .returning(move |_| Ok(vec![]));

        let result = mock_db
            .get_deleted_items_by_user("non_deleting_user")
            .await
            .unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_get_deleted_items_empty() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_get_deleted_items()
            .returning(move || Ok(vec![]));

        let result = mock_db.get_deleted_items().await.unwrap();
        assert_eq!(result.len(), 0);
    }
}

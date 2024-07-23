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
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub age: u32,
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
    async fn scan(&self) -> Result<Vec<Item>>;
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
    async fn scan(&self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut scan_output = self
                .client
                .scan()
                .table_name(&self.table_name)
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await?;

            if let Some(scanned_items) = scan_output.items {
                for item in scanned_items {
                    items.push(from_item(item)?);
                }
            }

            last_evaluated_key = scan_output.last_evaluated_key.take();

            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(items)
    }
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
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn create(&self, create_item: CreateItem) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let item = Item {
            id: id.clone(),
            name: create_item.name,
            age: create_item.age,
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

    async fn update(&self, item: Item) -> Result<()> {
        let dynamo_item = to_item(item)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_exists(id)")
            .send()
            .await?;

        Ok(())
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
                },
                Item {
                    id: "id2".to_string(),
                    name: "Name 2".to_string(),
                    age: 40,
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
}

use aws_sdk_dynamodb::{Client, Error};
use aws_config::meta::region::RegionProviderChain;
use serde::{Serialize, Deserialize};
use serde_dynamo::{to_item, from_item};
use aws_sdk_dynamodb::types::AttributeValue;
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub age: u32,
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait DynamoDbTrait {
    async fn get_item(&self, id: &str) -> Result<Option<Item>>;
    async fn put_new_item(&self, item: Item) -> Result<String>;
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

        Ok(Self {
            client,
            table_name,
        })
    }
}

#[async_trait]
impl DynamoDbTrait for DynamoDb {
    async fn get_item(&self, id: &str) -> Result<Option<Item>> {
        let key = HashMap::from([
            ("id".to_string(), AttributeValue::S(id.to_string())),
        ]);

        let result = self.client
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

    async fn put_new_item(&self, mut item: Item) -> Result<String> {
        item.id = Uuid::new_v4().to_string();

        let dynamo_item = to_item(item.clone())?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_not_exists(id)")
            .send()
            .await?;

        println!("Added item with id: {}", item.id);
        Ok(item.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_get_item() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_get_item()
            .with(eq("test_id"))
            .times(1)
            .returning(|_| Ok(Some(Item {
                id: "test_id".to_string(),
                name: "Test Name".to_string(),
                age: 30,
            })));

        let result = mock.get_item("test_id").await.unwrap();
        assert!(result.is_some());
        let item = result.unwrap();
        assert_eq!(item.id, "test_id");
        assert_eq!(item.name, "Test Name");
        assert_eq!(item.age, 30);
    }

    #[tokio::test]
    async fn test_put_item() {
        let mut mock = MockDynamoDbTrait::new();
        mock.expect_put_new_item()
            .with(function(|item: &Item| item.name == "Test Name" && item.age == 30))
            .times(1)
            .returning(|_| Ok("new_id".to_string()));

        let item = Item {
            id: "".to_string(), // ID will be generated
            name: "Test Name".to_string(),
            age: 30,
        };

        let result = mock.put_new_item(item).await.unwrap();
        assert_eq!(result, "new_id");
    }
}

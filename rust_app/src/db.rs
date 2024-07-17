use aws_sdk_dynamodb::{Client, Error};
use aws_config::meta::region::RegionProviderChain;
use serde::{Serialize, Deserialize};
use serde_dynamo::{to_item, from_item};
use aws_sdk_dynamodb::types::AttributeValue;
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone)]
pub struct DynamoDb {
    client: Client,
    table_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    id: String,
    name: String,
    age: u32,
    // Add other fields as needed
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

    pub async fn get_item(&self, id: &str) -> Result<Option<Item>> {
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

    pub async fn put_item(&self, mut item: Item) -> Result<String> {
        // Generate a new UUID
        item.id = Uuid::new_v4().to_string();

        let dynamo_item = to_item(item.clone())?;

        let request = self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_not_exists(id)");

        println!("Executing request to add item");

        let _response = request.send().await?;

        println!("Added item with id: {}", item.id);
        Ok(item.id)
    }
}

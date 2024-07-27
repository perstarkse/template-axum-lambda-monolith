use anyhow::Result;
use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::operation::update_item::UpdateItemError;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client, Error};
use axum::response::IntoResponse;
use axum::Json;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_dynamo::{from_item, to_item};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum OperationResult<T> {
    Success(Option<T>),
    ItemNotFound,
    ItemAlreadyExists,
    InvalidInput,
    InternalError(String),
}

impl<T> IntoResponse for OperationResult<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            OperationResult::Success(_) => unreachable!("Success should be handled manually"),
            OperationResult::ItemNotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Item not found" })),
            )
                .into_response(),
            OperationResult::ItemAlreadyExists => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "Item already exists" })),
            )
                .into_response(),
            OperationResult::InvalidInput => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid input" })),
            )
                .into_response(),
            OperationResult::InternalError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e })),
            )
                .into_response(),
        }
    }
}

#[async_trait]
pub trait SoftDeletable: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync {
    fn get_deleted_at(&self) -> &Option<String>;
}

#[async_trait]
pub trait DynamoDbOperations<T>: Send + Sync {
    async fn get_item(&self, id: String) -> OperationResult<T>;
    async fn create(&self, item: T) -> OperationResult<T>;
    async fn update(&self, item: T) -> OperationResult<T>;
    async fn delete(&self, id: String) -> OperationResult<T>;
    async fn soft_delete(&self, id: String, user_id: String) -> OperationResult<T>;
    async fn scan(&self) -> OperationResult<Vec<T>>;
    async fn get_deleted_items_by_user(&self, user_id: String) -> OperationResult<Vec<T>>;
    async fn get_deleted_items(&self) -> OperationResult<Vec<T>>;
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
    async fn get_item(&self, id: String) -> OperationResult<T> {
        let key = HashMap::from([("id".to_string(), AttributeValue::S(id))]);

        match self
            .client
            .get_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .send()
            .await
        {
            Ok(result) => match result.item {
                Some(item) => match from_item::<
                    HashMap<std::string::String, aws_sdk_dynamodb::types::AttributeValue>,
                    T,
                >(item)
                {
                    Ok(item) => {
                        if item.get_deleted_at().is_none() {
                            OperationResult::Success(Some(item))
                        } else {
                            OperationResult::ItemNotFound
                        }
                    }
                    Err(err) => OperationResult::InternalError(err.to_string()),
                },
                None => OperationResult::ItemNotFound,
            },
            Err(err) => OperationResult::InternalError(err.to_string()),
        }
    }

    async fn scan(&self) -> OperationResult<Vec<T>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            match self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("attribute_not_exists(deleted_at)")
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await
            {
                Ok(result) => {
                    if let Some(scanned_items) = result.items {
                        for item in scanned_items {
                            match from_item(item) {
                                Ok(item) => items.push(item),
                                Err(err) => return OperationResult::InternalError(err.to_string()),
                            }
                        }
                    }

                    last_evaluated_key = result.last_evaluated_key;

                    if last_evaluated_key.is_none() {
                        break;
                    }
                }
                Err(err) => return OperationResult::InternalError(err.to_string()),
            }
        }

        OperationResult::Success(Some(items))
    }

    async fn update(&self, item: T) -> OperationResult<T> {
        let dynamo_item = match to_item(item) {
            Ok(item) => item,
            Err(err) => return OperationResult::InternalError(err.to_string()),
        };

        match self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_exists(id) AND attribute_not_exists(deleted_at)")
            .send()
            .await
        {
            Ok(_) => OperationResult::Success(None),
            Err(err) => match err.into_service_error() {
                PutItemError::ConditionalCheckFailedException(_) => OperationResult::ItemNotFound,
                _ => OperationResult::InternalError("Service Error".to_string()),
            },
        }
    }

    async fn create(&self, item: T) -> OperationResult<T> {
        let dynamo_item = match to_item(item) {
            Ok(item) => item,
            Err(err) => return OperationResult::InternalError(err.to_string()),
        };

        match self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(dynamo_item))
            .condition_expression("attribute_not_exists(id)")
            .send()
            .await
        {
            Ok(_) => OperationResult::Success(None),
            Err(err) => match err.into_service_error() {
                PutItemError::ConditionalCheckFailedException(_) => {
                    OperationResult::ItemAlreadyExists
                }
                _ => OperationResult::InternalError("Service Error".to_string()),
            },
        }
    }

    async fn delete(&self, id: String) -> OperationResult<T> {
        let key = HashMap::from([("id".to_string(), AttributeValue::S(id))]);

        match self
            .client
            .delete_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .condition_expression("attribute_exists(id)")
            .send()
            .await
        {
            Ok(_) => OperationResult::Success(None),
            Err(err) => match err.into_service_error() {
                DeleteItemError::ConditionalCheckFailedException(_) => {
                    OperationResult::ItemNotFound
                }
                _ => OperationResult::InternalError("Service Error".to_string()),
            },
        }
    }

    async fn soft_delete(&self, id: String, user_id: String) -> OperationResult<T> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            .to_string();

        match self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .update_expression("SET deleted_at = :deleted_at, deleted_by = :deleted_by")
            .condition_expression("attribute_exists(id) AND attribute_not_exists(deleted_at)")
            .expression_attribute_values(":deleted_at", AttributeValue::S(now))
            .expression_attribute_values(":deleted_by", AttributeValue::S(user_id.to_string()))
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

    async fn get_deleted_items_by_user(&self, user_id: String) -> OperationResult<Vec<T>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            match self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("attribute_exists(deleted_at) AND deleted_by = :user_id")
                .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await
            {
                Ok(result) => {
                    if let Some(scanned_items) = result.items {
                        for item in scanned_items {
                            match from_item(item) {
                                Ok(item) => items.push(item),
                                Err(err) => return OperationResult::InternalError(err.to_string()),
                            }
                        }
                    }

                    last_evaluated_key = result.last_evaluated_key;

                    if last_evaluated_key.is_none() {
                        break;
                    }
                }
                Err(err) => return OperationResult::InternalError(err.to_string()),
            }
        }
        OperationResult::Success(Some(items))
    }

    async fn get_deleted_items(&self) -> OperationResult<Vec<T>> {
        let mut items = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            match self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("attribute_exists(deleted_at)")
                .set_exclusive_start_key(last_evaluated_key)
                .send()
                .await
            {
                Ok(result) => {
                    if let Some(scanned_items) = result.items {
                        for item in scanned_items {
                            match from_item(item) {
                                Ok(item) => items.push(item),
                                Err(err) => return OperationResult::InternalError(err.to_string()),
                            }
                        }
                    }

                    last_evaluated_key = result.last_evaluated_key;

                    if last_evaluated_key.is_none() {
                        break;
                    }
                }
                Err(err) => return OperationResult::InternalError(err.to_string()),
            }
        }
        OperationResult::Success(Some(items))
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
            async fn get_item(&self, id: String) -> OperationResult<TestItem>;
            async fn create(&self, item: TestItem) -> OperationResult<TestItem>;
            async fn update(&self, item: TestItem) -> OperationResult<TestItem>;
            async fn delete(&self, id: String) -> OperationResult<TestItem>;
            async fn soft_delete(&self, id: String, user_id: String) -> OperationResult<TestItem>;
            async fn scan(&self) -> OperationResult<Vec<TestItem>>;
            async fn get_deleted_items_by_user(&self, user_id: String) -> OperationResult<Vec<TestItem>>;
            async fn get_deleted_items(&self) -> OperationResult<Vec<TestItem>>;
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
            .with(eq("test_id".to_string()))
            .returning(move |_| OperationResult::Success(Some(test_item.clone())));

        let result = mock_db.get_item("test_id".to_string()).await;
        match result {
            OperationResult::Success(Some(item)) => assert_eq!(item.name, "test_name"),
            _ => panic!("Expected Success with item"),
        }
    }

    #[tokio::test]
    async fn test_get_item_not_found() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_get_item()
            .with(eq("non_existing_id".to_string()))
            .returning(|_| OperationResult::ItemNotFound);

        let result = mock_db.get_item("non_existing_id".to_string()).await;
        assert!(matches!(result, OperationResult::ItemNotFound));
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
            .returning(|_| OperationResult::Success(None));

        let result = mock_db.create(test_item).await;
        assert!(matches!(result, OperationResult::Success(None)));
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
            .returning(|_| OperationResult::ItemAlreadyExists);

        let result = mock_db.create(test_item).await;
        assert!(matches!(result, OperationResult::ItemAlreadyExists));
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

        mock_db
            .expect_update()
            .returning(|_| OperationResult::Success(None));

        let result = mock_db.update(test_item).await;
        assert!(matches!(result, OperationResult::Success(None)));
    }

    #[tokio::test]
    async fn test_delete_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_delete()
            .with(eq("delete_id".to_string()))
            .returning(|_| OperationResult::Success(None));

        let result = mock_db.delete("delete_id".to_string()).await;
        assert!(matches!(result, OperationResult::Success(None)));
    }

    #[tokio::test]
    async fn test_soft_delete_item() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_soft_delete()
            .with(eq("soft_delete_id".to_string()), eq("user_123".to_string()))
            .returning(|_, _| OperationResult::Success(None));

        let result = mock_db
            .soft_delete("soft_delete_id".to_string(), "user_123".to_string())
            .await;
        assert!(matches!(result, OperationResult::Success(None)));
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
            .returning(move || OperationResult::Success(Some(test_items.clone())));

        let result = mock_db.scan().await;
        match result {
            OperationResult::Success(Some(items)) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].name, "name1");
                assert_eq!(items[1].name, "name2");
            }
            _ => panic!("Expected Success with items"),
        }
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
            .with(eq("user_456".to_string()))
            .returning(move |_| OperationResult::Success(Some(deleted_items.clone())));

        let result = mock_db
            .get_deleted_items_by_user("user_456".to_string())
            .await;
        match result {
            OperationResult::Success(Some(items)) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].deleted_by, Some("user_456".to_string()));
                assert_eq!(items[1].deleted_by, Some("user_456".to_string()));
            }
            _ => panic!("Expected Success with items"),
        }
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
            .returning(move || OperationResult::Success(Some(deleted_items.clone())));

        let result = mock_db.get_deleted_items().await;
        match result {
            OperationResult::Success(Some(items)) => {
                assert_eq!(items.len(), 2);
                assert!(items[0].deleted_at.is_some());
                assert!(items[1].deleted_at.is_some());
            }
            _ => panic!("Expected Success with items"),
        }
    }

    #[tokio::test]
    async fn test_create_item_already_exists() {
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
            .returning(|_| OperationResult::ItemAlreadyExists);

        let result = mock_db.create(test_item).await;
        assert!(matches!(result, OperationResult::ItemAlreadyExists));
    }

    #[tokio::test]
    async fn test_soft_delete_item_already_deleted() {
        let mut mock_db = MockDynamoDbTestItem::new();

        let deleted_item = TestItem {
            id: "soft_delete_id".to_string(),
            name: "deleted_name".to_string(),
            age: 35,
            deleted_at: Some("2023-06-01".to_string()),
            deleted_by: Some("user_123".to_string()),
        };

        mock_db
            .expect_soft_delete()
            .with(eq("soft_delete_id".to_string()), eq("user_123".to_string()))
            .returning(move |_, _| OperationResult::Success(Some(deleted_item.clone())));

        let result = mock_db
            .soft_delete("soft_delete_id".to_string(), "user_123".to_string())
            .await;
        match result {
            OperationResult::Success(Some(item)) => {
                assert_eq!(item.deleted_by, Some("user_123".to_string()))
            }
            _ => panic!("Expected Success with item"),
        }
    }

    #[tokio::test]
    async fn test_scan_items_empty() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_scan()
            .returning(|| OperationResult::Success(Some(vec![])));

        let result = mock_db.scan().await;
        match result {
            OperationResult::Success(Some(items)) => assert!(items.is_empty()),
            _ => panic!("Expected Success with empty items"),
        }
    }

    #[tokio::test]
    async fn test_get_deleted_items_by_user_empty() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_get_deleted_items_by_user()
            .with(eq("user_456".to_string()))
            .returning(|_| OperationResult::Success(Some(vec![])));

        let result = mock_db
            .get_deleted_items_by_user("user_456".to_string())
            .await;
        match result {
            OperationResult::Success(Some(items)) => assert!(items.is_empty()),
            _ => panic!("Expected Success with empty items"),
        }
    }

    #[tokio::test]
    async fn test_get_deleted_items_empty() {
        let mut mock_db = MockDynamoDbTestItem::new();

        mock_db
            .expect_get_deleted_items()
            .returning(|| OperationResult::Success(Some(vec![])));

        let result = mock_db.get_deleted_items().await;
        match result {
            OperationResult::Success(Some(items)) => assert!(items.is_empty()),
            _ => panic!("Expected Success with empty items"),
        }
    }
}

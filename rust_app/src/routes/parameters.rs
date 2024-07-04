use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize)]
pub struct Params {
    first: Option<String>,
    second: Option<String>,
}

pub async fn handler(Query(params): Query<Params>) -> Json<Value> {
    Json(json!({ "request parameters": params }))
}


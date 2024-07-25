use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::debug;

#[derive(Deserialize, Serialize)]
pub struct Params {
    first: Option<String>,
    second: Option<String>,
}

pub async fn handler(Query(params): Query<Params>) -> Json<Value> {
    debug!("Params handler is run");
    Json(json!({ "request parameters": params }))
}

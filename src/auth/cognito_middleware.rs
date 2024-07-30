use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::cognito_auth::{Auth, AuthOperations};

pub async fn auth_middleware(
    State(state): State<Auth>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => match state.verify_token(token).await {
            Ok(claims) => {
                request.extensions_mut().insert(claims);
                next.run(request).await
            }
            Err(e) => e.into_response(),
        },
        None => next.run(request).await,
    }
}

// We can use the middleware in routes like the following:
// pub async fn delete(
//     Extension(db): Extension<DynamoDbRepository<Item>>,
//     Path(id): Path<String>,
//     claims: Option<Extension<Claims>>,
// ) -> Response {
//     match claims {
//         Some(claims) => match db.soft_delete(id, claims.username.clone()).await {
//             OperationResult::Success(_) => (
//                 StatusCode::NO_CONTENT,
//                 Json(json!({"message": "Item was successfully removed"})),
//             )
//                 .into_response(),
//             err => err.into_response(),
//         },
//         None => (
//             StatusCode::UNAUTHORIZED,
//             Json(json!({
//                 "message": "You are not authenticated",
//             })),
//         )
//             .into_response(),
//     }
// }

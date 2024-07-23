use crate::auth::{Auth, AuthTrait};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};

pub async fn auth_middleware(
    State(state): State<Auth>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => match state.verify_token(token).await {
            Ok(claims) => {
                request.extensions_mut().insert(claims);
                Ok(next.run(request).await)
            }
            Err(_e) => Ok(next.run(request).await),
        },
        None => Ok(next.run(request).await),
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::auth::{AuthError, Claims, MockAuthTrait};

//     use super::*;
//     use axum::body::Body;
//     use axum::http::{Request, StatusCode};
//     use axum::middleware::from_fn_with_state;
//     use axum::Router;
//     use lambda_http::tower::ServiceExt;
//     use mockall::predicate::*;

//     fn create_mock_claims() -> Claims {
//         Claims {
//             sub: "user123".to_string(),
//             exp: 1625097600,
//             client_id: "test_client".to_string(),
//             scope: "openid profile".to_string(),
//             token_use: "access".to_string(),
//             username: "testuser".to_string(),
//             auth_time: 1625011200,
//             iss: "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_example".to_string(),
//             iat: 1625011200,
//             jti: "example-jti".to_string(),
//             origin_jti: "example-origin-jti".to_string(),
//             event_id: "example-event-id".to_string(),
//         }
//     }

//     fn create_mock_app(mock_auth: MockAuthTrait) -> Router {
//         let mut mock = MockAuthTrait::new();
//         Router::new()
//             .route("/", axum::routing::get(|| async { "Hello, World!" }))
//             .layer(from_fn_with_state(mock_auth, auth_middleware))
//     }

//     #[tokio::test]
//     async fn test_auth_middleware_valid_token() {
//         let mut mock_auth = MockAuthTrait::new();
//         mock_auth
//             .expect_verify_token()
//             .with(eq("valid_token"))
//             .times(1)
//             .returning(|_| Ok(create_mock_claims()));

//         let app = create_mock_app(mock_auth);

//         let response = app
//             .oneshot(
//                 Request::builder()
//                     .uri("/")
//                     .header("Authorization", "Bearer valid_token")
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         assert_eq!(response.status(), StatusCode::OK);
//     }

//     #[tokio::test]
//     async fn test_auth_middleware_invalid_token() {
//         let mut mock_auth = MockAuthTrait::new();
//         mock_auth
//             .expect_verify_token()
//             .with(eq("invalid_token"))
//             .times(1)
//             .returning(|_| {
//                 Err(AuthError::JwtError(
//                     jsonwebtokens_cognito::Error::InvalidSignature(),
//                 ))
//             });

//         let app = create_mock_app(mock_auth);

//         let response = app
//             .oneshot(
//                 Request::builder()
//                     .uri("/")
//                     .header("Authorization", "Bearer invalid_token")
//                     .body(Body::empty())
//                     .unwrap(),
//             )
//             .await
//             .unwrap();

//         assert_eq!(response.status(), StatusCode::OK);
//     }

//     #[tokio::test]
//     async fn test_auth_middleware_no_token() {
//         let mock_auth = MockAuthTrait::new();
//         let app = create_mock_app(mock_auth);

//         let response = app
//             .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
//             .await
//             .unwrap();

//         assert_eq!(response.status(), StatusCode::OK);
//     }
// }

use async_trait::async_trait;
use axum::{response::IntoResponse, Json};
use jsonwebtokens_cognito::{Error as JwtError, KeySet};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use mockall::automock;
use serde_json::json;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,        // Subject identifier (unique user ID)
    pub exp: usize,         // Expiration time (Unix timestamp)
    pub client_id: String,  // ID of the client application
    pub scope: String,      // Permissions granted to the token
    pub token_use: String,  // Type of token (e.g., "access")
    pub username: String,   // Username (often same as sub)
    pub auth_time: usize,   // Time of authentication (Unix timestamp)
    pub iss: String,        // Issuer (Cognito user pool URL)
    pub iat: usize,         // Issued at time (Unix timestamp)
    pub jti: String,        // JWT ID (unique identifier for this token)
    pub origin_jti: String, // Original JWT ID
    pub event_id: String,   // Unique identifier for the authentication event
}

#[derive(Clone, Debug)]
pub struct Auth {
    keyset: KeySet,
    client_id: String,
}

#[derive(Debug)]
pub enum AuthError {
    InvalidSignature,
    TokenExpired,
    InvalidToken,
    MalformedToken,
    VerifierFailedBuilding(String),
    VerificationFailed(String),
    ConversionError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AuthError::InvalidSignature => (
                StatusCode::UNAUTHORIZED,
                Json(json!("Invalid token signature")),
            )
                .into_response(),
            AuthError::TokenExpired => {
                (StatusCode::UNAUTHORIZED, Json(json!("Token has expired"))).into_response()
            }
            AuthError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, Json(json!("Invalid token"))).into_response()
            }
            AuthError::MalformedToken => {
                (StatusCode::BAD_REQUEST, Json(json!("Malformed token"))).into_response()
            }
            AuthError::VerifierFailedBuilding(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(err))).into_response()
            }
            AuthError::VerificationFailed(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(err))).into_response()
            }
            AuthError::ConversionError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(err))).into_response()
            }
        }
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait AuthOperations {
    async fn verify_token(&self, token: &str) -> Result<Claims, AuthError>;
}

impl Auth {
    pub fn new(region: &str, user_pool_id: &str, client_id: &str) -> Result<Self, JwtError> {
        match KeySet::new(region, user_pool_id) {
            Ok(keyset) => Ok(Self {
                keyset,
                client_id: client_id.to_string(),
            }),
            Err(err) => Err(err),
        }
    }
}

#[async_trait]
impl AuthOperations for Auth {
    async fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        match self
            .keyset
            .new_access_token_verifier(&[&self.client_id])
            .build()
        {
            Ok(verifier) => match self.keyset.verify(token, &verifier).await {
                Ok(claims) => match serde_json::from_value(claims) {
                    Ok(claims) => Ok(claims),
                    Err(err) => Err(AuthError::ConversionError(err.to_string())),
                },
                Err(err) => match err {
                    JwtError::InvalidSignature() => Err(AuthError::InvalidSignature),
                    JwtError::TokenExpiredAt(_) => Err(AuthError::TokenExpired),
                    _ => Err(AuthError::VerificationFailed(err.to_string())),
                },
            },
            Err(err) => Err(AuthError::VerifierFailedBuilding(err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    fn create_mock_claims() -> Claims {
        Claims {
            sub: "user123".to_string(),
            exp: 1625097600,
            client_id: "test_client".to_string(),
            scope: "openid profile".to_string(),
            token_use: "access".to_string(),
            username: "testuser".to_string(),
            auth_time: 1625011200,
            iss: "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_example".to_string(),
            iat: 1625011200,
            jti: "example-jti".to_string(),
            origin_jti: "example-origin-jti".to_string(),
            event_id: "example-event-id".to_string(),
        }
    }

    #[tokio::test]
    async fn test_verify_valid_token() {
        let mut mock = MockAuthOperations::new();

        mock.expect_verify_token()
            .with(eq("valid_token"))
            .times(1)
            .returning(move |_| Ok(create_mock_claims()));

        let result = mock.verify_token("valid_token").await.unwrap();

        assert_eq!(result.sub, "user123");
        assert_eq!(result.username, "testuser");
        assert_eq!(result.client_id, "test_client");
    }

    #[tokio::test]
    async fn test_verify_invalid_token() {
        let mut mock = MockAuthOperations::new();

        mock.expect_verify_token()
            .with(eq("invalid_token"))
            .times(1)
            .returning(|_| Err(AuthError::InvalidSignature));

        let result = mock.verify_token("invalid_token").await;

        assert!(matches!(result, Err(AuthError::InvalidSignature)));
    }

    #[tokio::test]
    async fn test_verify_expired_token() {
        let mut mock = MockAuthOperations::new();

        mock.expect_verify_token()
            .with(eq("expired_token"))
            .times(1)
            .returning(|_| Err(AuthError::TokenExpired));

        let result = mock.verify_token("expired_token").await;

        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[tokio::test]
    async fn test_verify_token_parsing_error() {
        let mut mock = MockAuthOperations::new();

        mock.expect_verify_token()
            .with(eq("malformed_token"))
            .times(1)
            .returning(|_| Err(AuthError::ConversionError("error".to_string())));

        let result = mock.verify_token("malformed_token").await;

        assert!(matches!(result, Err(AuthError::ConversionError(_))));
    }
}

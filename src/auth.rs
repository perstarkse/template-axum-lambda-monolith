use async_trait::async_trait;
use jsonwebtokens_cognito::{Error as JwtError, KeySet};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use mockall::automock;

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

#[derive(Clone)]
pub struct Auth {
    keyset: KeySet,
    client_id: String,
}

#[derive(Debug)]
pub enum AuthError {
    JwtError(JwtError),
    ParsingError(serde_json::Error),
}

impl From<JwtError> for AuthError {
    fn from(err: JwtError) -> Self {
        AuthError::JwtError(err)
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(err: serde_json::Error) -> Self {
        AuthError::ParsingError(err)
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait AuthTrait {
    async fn verify_token(&self, token: &str) -> Result<Claims, AuthError>;
}

impl Auth {
    pub fn new(region: &str, user_pool_id: &str, client_id: &str) -> Result<Self, JwtError> {
        let keyset = KeySet::new(region, user_pool_id)?;
        Ok(Self {
            keyset,
            client_id: client_id.to_string(),
        })
    }
}

#[async_trait]
impl AuthTrait for Auth {
    async fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let verifier = self
            .keyset
            .new_access_token_verifier(&[&self.client_id])
            .build()
            .map_err(|e| AuthError::JwtError(e.into()))?;

        let claims = self.keyset.verify(token, &verifier).await?;
        let claims: Claims = serde_json::from_value(claims)?;

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use serde::de::Error;

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
        let mut mock = MockAuthTrait::new();

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
        let mut mock = MockAuthTrait::new();

        mock.expect_verify_token()
            .with(eq("invalid_token"))
            .times(1)
            .returning(|_| Err(AuthError::JwtError(JwtError::InvalidSignature())));

        let result = mock.verify_token("invalid_token").await;

        assert!(matches!(
            result,
            Err(AuthError::JwtError(JwtError::InvalidSignature()))
        ));
    }

    #[tokio::test]
    async fn test_verify_expired_token() {
        let mut mock = MockAuthTrait::new();

        mock.expect_verify_token()
            .with(eq("expired_token"))
            .times(1)
            .returning(|_| Err(AuthError::JwtError(JwtError::TokenExpiredAt(0))));

        let result = mock.verify_token("expired_token").await;

        assert!(matches!(
            result,
            Err(AuthError::JwtError(JwtError::TokenExpiredAt(0)))
        ));
    }

    #[tokio::test]
    async fn test_verify_token_parsing_error() {
        let mut mock = MockAuthTrait::new();

        mock.expect_verify_token()
            .with(eq("malformed_token"))
            .times(1)
            .returning(|_| Err(AuthError::ParsingError(serde_json::Error::custom("error"))));

        let result = mock.verify_token("malformed_token").await;

        assert!(matches!(result, Err(AuthError::ParsingError(_))));
    }
}

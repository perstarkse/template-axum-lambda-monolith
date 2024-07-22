use jsonwebtokens_cognito::{Error as JwtError, KeySet};
use serde::{Deserialize, Serialize};

/// TODO
/// Implement caching, look at the jsonwebtokens_cognito crate

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,            // Subject identifier (unique user ID)
    pub exp: usize,             // Expiration time (Unix timestamp)
    pub client_id: String,      // ID of the client application
    pub scope: String,          // Permissions granted to the token
    pub token_use: String,      // Type of token (e.g., "access")
    pub username: String,       // Username (often same as sub)
    pub auth_time: usize,       // Time of authentication (Unix timestamp)
    pub iss: String,            // Issuer (Cognito user pool URL)
    pub iat: usize,             // Issued at time (Unix timestamp)
    pub jti: String,            // JWT ID (unique identifier for this token)
    pub origin_jti: String,     // Original JWT ID
    pub event_id: String,       // Unique identifier for the authentication event
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

impl Auth {
    pub fn new(region: &str, user_pool_id: &str, client_id: &str) -> Result<Self, JwtError> {
        let keyset = KeySet::new(region, user_pool_id)?;
        Ok(Self { keyset, client_id: client_id.to_string() })
    }

    pub async fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
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

use jsonwebtokens_cognito::{Error as JwtError, KeySet};
use serde::{Deserialize, Serialize};
/// TODO
/// Implement caching, look at the jsonwebtokens_cognito crate

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
}

pub struct Auth {
    keyset: KeySet,
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
    pub fn new(region: &str, user_pool_id: &str) -> Result<Self, JwtError> {
        let keyset = KeySet::new(region, user_pool_id)?;
        Ok(Self { keyset })
    }

    pub async fn verify_token(&self, token: &str, client_id: &str) -> Result<Claims, AuthError> {
        let verifier = self
            .keyset
            .new_id_token_verifier(&[client_id])
            .build()
            .map_err(|e| AuthError::JwtError(e.into()))?;

        let claims = self.keyset.verify(token, &verifier).await?;

        // Parse the claims into our Claims struct
        let claims: Claims = serde_json::from_value(claims)?;

        Ok(claims)
    }
}

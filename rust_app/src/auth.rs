use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    extract::State,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest;
use serde_json::Value;
use std::time::{Duration, Instant};
use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    // Add other claims as needed
}

struct CachedKey {
    key: DecodingKey,
    expiry: Instant,
}

pub struct AuthState {
    config: Config,
    cached_key: RwLock<Option<CachedKey>>,
}

impl AuthState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cached_key: RwLock::new(None),
        }
    }

    async fn get_decoding_key(&self) -> Result<DecodingKey, reqwest::Error> {
        let mut cached_key = self.cached_key.write().await;
        
        if let Some(ref key) = *cached_key {
            if key.expiry > Instant::now() {
                return Ok(key.key.clone());
            }
        }

        let new_key = self.fetch_cognito_public_key().await?;
        *cached_key = Some(CachedKey {
            key: new_key.clone(),
            expiry: Instant::now() + Duration::from_secs(3600), // Cache for 1 hour
        });

        Ok(new_key)
    }

    async fn fetch_cognito_public_key(&self) -> Result<DecodingKey, reqwest::Error> {
        let jwks_url = format!(
            "https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json",
            self.config.aws_region, self.config.cognito_user_pool_id
        );
        let resp: Value = reqwest::get(&jwks_url).await?.json().await?;
        
        // Extract the public key from the JWKS response
        // This is a simplified example; you should handle multiple keys and key rotation
        let n = resp["keys"][0]["n"].as_str().unwrap();
        let e = resp["keys"][0]["e"].as_str().unwrap();
        
        Ok(DecodingKey::from_rsa_components(n, e))
    }
}

pub async fn auth_middleware<B>(
    State(state): State<Arc<AuthState>>,
    req: Request<B>,
    next: Next<>,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let decoding_key = state.get_decoding_key().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[&state.config.cognito_app_client_id]);
    validation.set_issuer(&[&format!("https://cognito-idp.{}.amazonaws.com/{}", 
                                     state.config.aws_region, 
                                     state.config.cognito_user_pool_id)]);

    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            // You can add the claims to the request extensions if needed
            req.extensions_mut().insert(token_data.claims);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

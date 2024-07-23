use crate::auth::{Auth, AuthError, AuthTrait};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;

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
            Err(e) => {
                let (status, message) = match e {
                    AuthError::JwtError(jwt_error) => match jwt_error {
                        jsonwebtokens_cognito::Error::InvalidSignature() => {
                            (StatusCode::UNAUTHORIZED, "Invalid token signature")
                        }
                        jsonwebtokens_cognito::Error::TokenExpiredAt(_) => {
                            (StatusCode::UNAUTHORIZED, "Token has expired")
                        }
                        _ => (StatusCode::UNAUTHORIZED, "Invalid token"),
                    },
                    AuthError::ParsingError(_) => (StatusCode::BAD_REQUEST, "Malformed token"),
                };
                Err((status, Json(json!({ "error": message }))))
            }
        },
        None => Ok(next.run(request).await),
    }
}

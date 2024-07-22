use axum::{
    extract::Request, http::StatusCode, middleware::Next, response::Response, Extension, Json
};
use serde_json::json;
use crate::auth::Auth;

pub async fn auth_middleware(
    Extension(auth): Extension<Auth>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = request.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            match auth.verify_token(token).await {
                Ok(claims) => {
                    // Add the verified claims to the request extensions
                    request.extensions_mut().insert(claims);
                    Ok(next.run(request).await)
                },
                Err(e) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": format!("Invalid token: {:?}", e)}))
                )),
            }
        },
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "No token provided"}))
        )),
    }
}


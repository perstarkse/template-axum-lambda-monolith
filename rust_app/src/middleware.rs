use axum::{
    extract::{Request, State}, http::StatusCode, middleware::Next, response::Response, Extension, Json
};
use serde_json::json;
use crate::auth::Auth;

#[derive(Clone)]
pub struct AuthState {
    pub auth: Auth,
    pub require_auth: bool,
}

pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = request.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            match state.auth.verify_token(token).await {
                Ok(claims) => {
                    request.extensions_mut().insert(claims);
                    Ok(next.run(request).await)
                },
                Err(e) => {
                    if state.require_auth {
                        Err((
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"error": format!("Invalid token: {:?}", e)}))
                        ))
                    } else {
                        Ok(next.run(request).await)
                    }
                },
            }
        },
        None => {
            if state.require_auth {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "No token provided"}))
                ))
            } else {
                Ok(next.run(request).await)
            }
        },
    }
}

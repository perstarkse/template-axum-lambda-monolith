use crate::auth::{Auth, AuthOperations};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

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

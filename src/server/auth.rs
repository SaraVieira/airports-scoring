use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Middleware that checks the `X-API-Key` header against the `API_KEY` env var.
///
/// If `API_KEY` is unset or empty (dev mode), all requests are allowed through.
/// Returns `401 Unauthorized` when the key is missing or does not match.
pub async fn require_api_key(req: Request, next: Next) -> Result<Response, StatusCode> {
    let expected = std::env::var("API_KEY").unwrap_or_default();

    // Dev mode: env var unset or empty — allow everything.
    if expected.is_empty() {
        return Ok(next.run(req).await);
    }

    let provided = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if provided != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

/// Middleware that checks the `X-Admin-Password` header against the `ADMIN_PASSWORD` env var.
///
/// If `ADMIN_PASSWORD` is unset or empty (dev mode), all requests are allowed through.
/// Returns `403 Forbidden` when the password is missing or does not match.
pub async fn require_admin(req: Request, next: Next) -> Result<Response, StatusCode> {
    let expected = std::env::var("ADMIN_PASSWORD").unwrap_or_default();

    // Dev mode: env var unset or empty — allow everything.
    if expected.is_empty() {
        return Ok(next.run(req).await);
    }

    let provided = req
        .headers()
        .get("X-Admin-Password")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if provided != expected {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}

use crate::server::ServerContext;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn update_metrics(ctx: ServerContext, req: Request, next: Next) -> Response {
    let req_scheme = request_scheme(&req);
    let req_method = req.method().clone();
    let _guard = ctx.metrics.active_request(&req_method, req_scheme);
    next.run(req).await
}

fn request_scheme(req: &Request) -> &'static str {
    let forwarded_scheme = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim);

    match forwarded_scheme.or_else(|| req.uri().scheme_str()) {
        Some(scheme) if scheme.eq_ignore_ascii_case("https") => "https",
        _ => "http",
    }
}

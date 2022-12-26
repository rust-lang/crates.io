use axum::extract::MatchedPath;
use axum::middleware::Next;
use axum::response::Response;
use http::Request;

pub async fn set_transaction<B>(
    matched_path: Option<MatchedPath>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    if let Some(matched_path) = matched_path {
        let tx_name = format!("{} {}", request.method(), matched_path.as_str());
        sentry::configure_scope(|scope| scope.set_transaction(Some(&tx_name)));
    }

    next.run(request).await
}

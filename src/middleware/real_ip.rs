use crate::real_ip::process_xff_headers;
use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use std::net::{IpAddr, SocketAddr};

#[derive(Copy, Clone, Debug, Deref)]
pub struct RealIp(IpAddr);

pub async fn middleware(
    ConnectInfo(socket_addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> impl IntoResponse {
    let xff_ip = process_xff_headers(req.headers());
    let real_ip = xff_ip.unwrap_or_else(|| socket_addr.ip());

    req.extensions_mut().insert(RealIp(real_ip));

    next.run(req).await
}

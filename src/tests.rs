use conduit::{box_error, Body, Handler, HandlerResult, RequestExt, Response, StatusCode};
use futures_util::future::Future;
use hyper::{body::to_bytes, service::Service};

use super::service::{BlockingHandler, ServiceError};
use super::HyperResponse;

struct OkResult;
impl Handler for OkResult {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        Response::builder()
            .header("ok", "value")
            .body(Body::from_static(b"Hello, world!"))
            .map_err(box_error)
    }
}

struct ErrorResult;
impl Handler for ErrorResult {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        let error = ::std::io::Error::last_os_error();
        Err(Box::new(error))
    }
}

struct Panic;
impl Handler for Panic {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        panic!()
    }
}

struct InvalidHeader;
impl Handler for InvalidHeader {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        Response::builder()
            .header("invalid-value", "\r\n")
            .body(Body::from_static(b"discarded"))
            .map_err(box_error)
    }
}

struct InvalidStatus;
impl Handler for InvalidStatus {
    fn call(&self, _req: &mut dyn RequestExt) -> HandlerResult {
        Response::builder()
            .status(1000)
            .body(Body::empty())
            .map_err(box_error)
    }
}

struct AssertPathNormalized;
impl Handler for AssertPathNormalized {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        if req.path() == "/normalized" {
            OkResult.call(req)
        } else {
            ErrorResult.call(req)
        }
    }
}

struct Sleep;
impl Handler for Sleep {
    fn call(&self, req: &mut dyn RequestExt) -> HandlerResult {
        std::thread::sleep(std::time::Duration::from_millis(100));
        OkResult.call(req)
    }
}

fn make_service<H: Handler>(
    handler: H,
) -> impl Service<
    hyper::Request<hyper::Body>,
    Response = HyperResponse,
    Future = impl Future<Output = Result<HyperResponse, ServiceError>> + Send + 'static,
    Error = ServiceError,
> {
    use hyper::service::service_fn;

    let handler = std::sync::Arc::new(BlockingHandler::new(handler));

    service_fn(move |request: hyper::Request<hyper::Body>| {
        let remote_addr = ([0, 0, 0, 0], 0).into();
        handler.clone().blocking_handler(request, remote_addr)
    })
}

async fn simulate_request<H: Handler>(handler: H) -> HyperResponse {
    let mut service = make_service(handler);
    service.call(hyper::Request::default()).await.unwrap()
}

async fn assert_generic_err(resp: HyperResponse) {
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(resp.headers().is_empty());
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Internal Server Error");
}

#[tokio::test]
async fn valid_ok_response() {
    let resp = simulate_request(OkResult).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().len(), 1);
    let full_body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&*full_body, b"Hello, world!");
}

#[tokio::test]
async fn invalid_ok_responses() {
    assert_generic_err(simulate_request(InvalidHeader).await).await;
    assert_generic_err(simulate_request(InvalidStatus).await).await;
}

#[tokio::test]
async fn err_responses() {
    assert_generic_err(simulate_request(ErrorResult).await).await;
}

#[ignore] // catch_unwind not yet implemented
#[tokio::test]
async fn recover_from_panic() {
    assert_generic_err(simulate_request(Panic).await).await;
}

#[tokio::test]
async fn normalize_path() {
    let mut service = make_service(AssertPathNormalized);
    let req = hyper::Request::put("//removed/.././.././normalized")
        .body(hyper::Body::default())
        .unwrap();
    let resp = service.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().len(), 1);

    let req = hyper::Request::put("//normalized")
        .body(hyper::Body::default())
        .unwrap();
    let resp = service.call(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().len(), 1);
}

#[tokio::test]
async fn sleeping_doesnt_block_another_request() {
    let mut service = make_service(Sleep);

    let first = service.call(hyper::Request::default());
    let second = service.call(hyper::Request::default());

    let start = std::time::Instant::now();

    // Spawn 2 requests that each sleeps for 100ms
    let (first, second) = futures_util::join!(first, second);

    // Elapsed time should be closer to 100ms than 200ms
    assert!(start.elapsed().as_millis() < 150);

    assert_eq!(first.unwrap().status(), StatusCode::OK);
    assert_eq!(second.unwrap().status(), StatusCode::OK);
}

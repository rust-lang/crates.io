use cargo_registry::util::{CargoError, CargoResult};
use std::error::Error;
use std::fmt;

pub struct Response {
    inner: conduit::Response,
    body: String,
}

impl Response {
    pub(super) fn new(mut inner: conduit::Response) -> Result<Self, ResponseError> {
        use ResponseError::*;

        let mut body = Vec::new();
        inner.body.write_body(&mut body).unwrap();

        let resp = Response {
            inner,
            body: String::from_utf8(body).unwrap(),
        };

        match resp.status() {
            400...499 => Err(BadRequest(resp)),
            500...599 => Err(ServerError(resp)),
            _ => Ok(resp),
        }
    }

    pub fn status(&self) -> u32 {
        self.inner.status.0
    }

    pub fn text(&self) -> &str {
        &self.body
    }
}

pub enum ResponseError {
    MiddlewareError(Box<dyn CargoError>),
    BadRequest(Response),
    ServerError(Response),
}

impl From<Box<dyn Error + Send>> for ResponseError {
    fn from(e: Box<dyn Error + Send>) -> Self {
        ResponseError::MiddlewareError(CargoError::from_std_error(e))
    }
}

impl fmt::Debug for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ResponseError::*;
        match self {
            MiddlewareError(e) => write!(f, "MiddlewareError({:?})", e),
            BadRequest(_) => write!(f, "BadRequest(_)"),
            ServerError(_) => write!(f, "ServerError(_)"),
        }
    }
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ResponseError::*;
        match self {
            MiddlewareError(e) => write!(f, "middleware error: {}", e),
            BadRequest(e) => write!(f, "bad request: {}", e.text()),
            ServerError(e) => write!(f, "server error: {}", e.text()),
        }
    }
}

impl Error for ResponseError {}

pub trait ResultExt {
    fn allow_errors(self) -> CargoResult<Response>;
}

impl ResultExt for Result<Response, ResponseError> {
    fn allow_errors(self) -> CargoResult<Response> {
        use ResponseError::*;
        self.or_else(|e| match e {
            MiddlewareError(e) => Err(e),
            BadRequest(r) | ServerError(r) => Ok(r),
        })
    }
}

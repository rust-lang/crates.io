use std::error::{FromError, Error};
use std::fmt::{Show, Formatter};
use std::fmt;

use conduit::Response;

use util::json_response;

#[derive(RustcEncodable)] struct StringError { detail: String }
#[derive(RustcEncodable)] struct Bad { errors: Vec<StringError> }

// =============================================================================
// CargoError trait

pub trait CargoError: Send {
    fn description(&self) -> &str;
    fn detail(&self) -> Option<String> { None }
    fn cause<'a>(&'a self) -> Option<&'a (CargoError)> { None }

    fn response(&self) -> Option<Response> {
        if self.human() {
            Some(json_response(&Bad {
                errors: vec![StringError { detail: self.description().to_string() }]
            }))
        } else {
            self.cause().and_then(|cause| cause.response())
        }
    }
    fn human(&self) -> bool { false }
}

impl fmt::String for CargoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.description()));

        match self.detail() {
            Some(s) => try!(write!(f, "\n  {}", s)),
            None => {}
        }

        match self.cause() {
            Some(cause) => try!(write!(f, "\nCaused by: {}", cause)),
            None => {}
        }

        Ok(())
    }
}
impl fmt::Show for CargoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt::String::fmt(self, f)
    }
}

impl CargoError for Box<CargoError> {
    fn description(&self) -> &str { (**self).description() }
    fn detail(&self) -> Option<String> { (**self).detail() }
    fn cause(&self) -> Option<&CargoError> { (**self).cause() }
    fn human(&self) -> bool { (**self).human() }
    fn response(&self) -> Option<Response> { (**self).response() }
}
impl<T: CargoError> CargoError for Box<T> {
    fn description(&self) -> &str { (**self).description() }
    fn detail(&self) -> Option<String> { (**self).detail() }
    fn cause(&self) -> Option<&CargoError> { (**self).cause() }
    fn human(&self) -> bool { (**self).human() }
    fn response(&self) -> Option<Response> { (**self).response() }
}

pub type CargoResult<T> = Result<T, Box<CargoError>>;

// =============================================================================
// Chaining errors

pub trait ChainError<T> {
    fn chain_error<E, F>(self, callback: F) -> CargoResult<T>
                         where E: CargoError, F: FnOnce() -> E;
}

struct ChainedError<E> {
    error: E,
    cause: Box<CargoError>,
}

impl<T, F> ChainError<T> for F where F: FnOnce() -> CargoResult<T> {
    fn chain_error<E, C>(self, callback: C) -> CargoResult<T>
                         where E: CargoError, C: FnOnce() -> E {
        self().chain_error(callback)
    }
}

impl<T, E: CargoError> ChainError<T> for Result<T, E> {
    fn chain_error<E2, C>(self, callback: C) -> CargoResult<T>
                         where E2: CargoError, C: FnOnce() -> E2 {
        self.map_err(move |err| {
            Box::new(ChainedError {
                error: callback(),
                cause: Box::new(err),
            }) as Box<CargoError>
        })
    }
}

impl<T> ChainError<T> for Option<T> {
    fn chain_error<E, C>(self, callback: C) -> CargoResult<T>
                         where E: CargoError, C: FnOnce() -> E {
        match self {
            Some(t) => Ok(t),
            None => Err(Box::new(callback()) as Box<CargoError>),
        }
    }
}

impl<E: CargoError> CargoError for ChainedError<E> {
    fn description(&self) -> &str { self.error.description() }
    fn detail(&self) -> Option<String> { self.error.detail() }
    fn cause(&self) -> Option<&CargoError> { Some(&*self.cause) }
    fn response(&self) -> Option<Response> { self.error.response() }
    fn human(&self) -> bool { self.error.human() }
}

// =============================================================================
// Error impls

impl<E: Error + Send> CargoError for E {
    fn description(&self) -> &str { Error::description(self) }
    fn detail(&self) -> Option<String> { Error::detail(self) }
}

impl<E: Error + Send> FromError<E> for Box<CargoError> {
    fn from_error(err: E) -> Box<CargoError> {
        Box::new(err) as Box<CargoError>
    }
}

// =============================================================================
// Concrete errors

struct ConcreteCargoError {
    description: String,
    detail: Option<String>,
    cause: Option<Box<CargoError>>,
    human: bool,
}

impl Show for ConcreteCargoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl CargoError for ConcreteCargoError {
    fn description(&self) -> &str { self.description.as_slice() }
    fn detail(&self) -> Option<String> { self.detail.clone() }
    fn cause(&self) -> Option<&CargoError> { self.cause.as_ref().map(|c| &**c) }
    fn human(&self) -> bool { self.human }
}

pub struct NotFound;

impl CargoError for NotFound {
    fn description(&self) -> &str { "not found" }

    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError { detail: "Not Found".to_string() }],
        });
        response.status = (404, "Not Found");
        return Some(response);
    }
}

pub struct Unauthorized;

impl CargoError for Unauthorized {
    fn description(&self) -> &str { "unauthorized" }

    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: "must be logged in to perform that action".to_string(),
            }],
        });
        response.status = (403, "Forbidden");
        return Some(response);
    }
}

pub fn internal_error<S1: Str, S2: Str>(error: S1,
                                        detail: S2) -> Box<CargoError> {
    Box::new(ConcreteCargoError {
        description: error.as_slice().to_string(),
        detail: Some(detail.as_slice().to_string()),
        cause: None,
        human: false,
    }) as Box<CargoError>
}

pub fn internal<S: fmt::String>(error: S) -> Box<CargoError> {
    Box::new(ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: false,
    }) as Box<CargoError>
}

pub fn human<S: fmt::String>(error: S) -> Box<CargoError> {
    Box::new(ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: true,
    }) as Box<CargoError>
}

pub fn std_error(e: Box<CargoError>) -> Box<Error+Send> {
    struct E(Box<CargoError>);
    impl Error for E {
        fn description(&self) -> &str { self.0.description() }
        fn detail(&self) -> Option<String> { Some(self.0.to_string()) }
    }
    Box::new(E(e))
}

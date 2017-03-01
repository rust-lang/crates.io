use std::error::Error;
use std::fmt;

use conduit::Response;

use util::json_response;

#[derive(RustcEncodable)] struct StringError { detail: String }
#[derive(RustcEncodable)] struct Bad { errors: Vec<StringError> }

// =============================================================================
// CargoError trait

pub trait CargoError: Send + fmt::Display + 'static {
    fn description(&self) -> &str;
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

impl fmt::Debug for Box<CargoError> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl CargoError for Box<CargoError> {
    fn description(&self) -> &str { (**self).description() }
    fn cause(&self) -> Option<&CargoError> { (**self).cause() }
    fn human(&self) -> bool { (**self).human() }
    fn response(&self) -> Option<Response> { (**self).response() }
}
impl<T: CargoError> CargoError for Box<T> {
    fn description(&self) -> &str { (**self).description() }
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
    #[allow(trivial_casts)]
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
            None => Err(Box::new(callback())),
        }
    }
}

impl<E: CargoError> CargoError for ChainedError<E> {
    fn description(&self) -> &str { self.error.description() }
    fn cause(&self) -> Option<&CargoError> { Some(&*self.cause) }
    fn response(&self) -> Option<Response> { self.error.response() }
    fn human(&self) -> bool { self.error.human() }
}

impl<E: CargoError> fmt::Display for ChainedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} caused by {}", self.error, self.cause)
    }
}

// =============================================================================
// Error impls

impl<E: Error + Send + 'static> From<E> for Box<CargoError> {
    fn from(err: E) -> Box<CargoError> {
        struct Shim<E>(E);
        impl<E: Error + Send + 'static> CargoError for Shim<E> {
            fn description(&self) -> &str { Error::description(&self.0) }
        }
        impl<E: fmt::Display> fmt::Display for Shim<E> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        Box::new(Shim(err))
    }
}

impl CargoError for ::curl::Error {
    fn description(&self) -> &str { Error::description(self) }
}
impl CargoError for ::rustc_serialize::json::DecoderError {
    fn description(&self) -> &str { Error::description(self) }
}

// =============================================================================
// Concrete errors

struct ConcreteCargoError {
    description: String,
    detail: Option<String>,
    cause: Option<Box<CargoError>>,
    human: bool,
}

impl fmt::Display for ConcreteCargoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)?;
        match self.detail  {
            Some(ref s) => write!(f, " ({})", s)?,
            None => {}
        }
        Ok(())
    }
}

impl CargoError for ConcreteCargoError {
    fn description(&self) -> &str { &self.description }
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

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Not Found".fmt(f)
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

impl fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "must be logged in to perform that action".fmt(f)
    }
}

pub fn internal_error(error: &str, detail: &str) -> Box<CargoError> {
    Box::new(ConcreteCargoError {
        description: error.to_string(),
        detail: Some(detail.to_string()),
        cause: None,
        human: false,
    })
}

pub fn internal<S: fmt::Display>(error: S) -> Box<CargoError> {
    Box::new(ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: false,
    })
}

pub fn human<S: fmt::Display>(error: S) -> Box<CargoError> {
    Box::new(ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: true,
    })
}

pub fn std_error(e: Box<CargoError>) -> Box<Error+Send> {
    #[derive(Debug)]
    struct E(Box<CargoError>);
    impl Error for E {
        fn description(&self) -> &str { self.0.description() }
    }
    impl fmt::Display for E {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)?;

            let mut err = &*self.0;
            while let Some(cause) = err.cause() {
                err = cause;
                write!(f, "\nCaused by: {}", err)?;
            }

            Ok(())
        }
    }
    Box::new(E(e))
}

use std::io::IoError;
use std::fmt;
use std::fmt::{Show, Formatter};
use std::fmt::Error as FormatError;

use conduit::Response;
use curl::ErrCode;
use pg::Error as PostgresError;
use pg::ConnectError;
use rustc_serialize::json;
use git2;

use util::json_response;

#[derive(RustcEncodable)] struct Error { detail: String }
#[derive(RustcEncodable)] struct Bad { errors: Vec<Error> }

pub trait CargoError: Send {
    fn description(&self) -> String;
    fn detail(&self) -> Option<String> { None }
    fn cause<'a>(&'a self) -> Option<&'a (CargoError)> { None }

    fn concrete(&self) -> ConcreteCargoError {
        ConcreteCargoError {
            description: self.description(),
            detail: self.detail(),
            cause: self.cause().map(|c| box c.concrete() as Box<CargoError>),
            human: false,
        }
    }

    fn response(&self) -> Option<Response> {
        if self.human() {
            Some(json_response(&Bad {
                errors: vec![Error { detail: self.description() }]
            }))
        } else {
            self.cause().and_then(|cause| cause.response())
        }
    }
    fn human(&self) -> bool { false }
}

pub trait FromError<E> {
    fn from_error(error: E) -> Self;
}

impl<E: CargoError> FromError<E> for Box<CargoError> {
    fn from_error(error: E) -> Box<CargoError> {
        box error as Box<CargoError>
    }
}

macro_rules! from_error {
    ($ty:ty) => {
        impl FromError<$ty> for $ty {
            fn from_error(error: $ty) -> $ty {
                error
            }
        }
    }
}

impl<'a> Show for &'a (CargoError) {
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

impl Show for Box<CargoError> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let me: &(CargoError) = &**self;
        me.fmt(f)
    }
}

impl CargoError for Box<CargoError> {
    fn description(&self) -> String { (**self).description() }
    fn detail(&self) -> Option<String> { (**self).detail() }
    fn cause<'a>(&'a self) -> Option<&'a (CargoError)> { (**self).cause() }
    fn human(&self) -> bool { (**self).human() }
    fn response(&self) -> Option<Response> { (**self).response() }
}

pub type CargoResult<T> = Result<T, Box<CargoError>>;

pub trait ChainError<T> {
    fn chain_error<E: CargoError, F>(self, callback: F) -> CargoResult<T>
        where F: FnOnce() -> E;
}

impl<'a, T, F> ChainError<T> for F where F: FnOnce() -> CargoResult<T> + 'a {
    fn chain_error<E: CargoError, F>(self, callback: F) -> CargoResult<T>
        where F: FnOnce() -> E
    {
        self().map_err(move |err| callback().concrete().with_cause(err))
    }
}

impl<T, E: CargoError> ChainError<T> for Result<T, E> {
    fn chain_error<E: CargoError, F>(self, callback: F) -> CargoResult<T>
        where F: FnOnce() -> E
    {
        self.map_err(move |err| callback().concrete().with_cause(err))
    }
}

impl CargoError for IoError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(IoError);

impl CargoError for FormatError {
    fn description(&self) -> String {
        "formatting failed".to_string()
    }
}

from_error!(FormatError);

impl CargoError for PostgresError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(PostgresError);

impl CargoError for ConnectError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(ConnectError);

impl CargoError for ErrCode {
    fn description(&self) -> String { self.to_string() }
}

from_error!(ErrCode);

impl CargoError for json::DecoderError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(json::DecoderError);

impl CargoError for git2::Error {
    fn description(&self) -> String { self.to_string() }
}

from_error!(git2::Error);

impl<T: CargoError> FromError<T> for Box<Show + 'static> {
    fn from_error(t: T) -> Box<Show + 'static> {
        box() (box t as Box<CargoError>) as Box<Show + 'static>
    }
}

pub struct ConcreteCargoError {
    description: String,
    detail: Option<String>,
    cause: Option<Box<CargoError>>,
    human: bool,
}

impl ConcreteCargoError {
    pub fn with_cause<E>(mut self, cause: E) -> Box<CargoError>
                        where E: CargoError {
        self.cause = Some(box cause as Box<CargoError>);
        box self as Box<CargoError>
    }
}

impl Show for ConcreteCargoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl CargoError for ConcreteCargoError {
    fn description(&self) -> String {
        self.description.clone()
    }

    fn detail(&self) -> Option<String> {
        self.detail.clone()
    }

    fn cause<'a>(&'a self) -> Option<&'a (CargoError)> {
        self.cause.as_ref().map(|c| { let err: &(CargoError) = &**c; err })
    }

    fn human(&self) -> bool { self.human }
}

pub struct NotFound;

impl CargoError for NotFound {
    fn description(&self) -> String { "not found".to_string() }

    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![Error { detail: "Not Found".to_string() }],
        });
        response.status = (404, "Not Found");
        return Some(response);
    }
}

from_error!(NotFound);

pub struct Unauthorized;

impl CargoError for Unauthorized {
    fn description(&self) -> String { "unauthorized".to_string() }

    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![Error {
                detail: "must be logged in to perform that action".to_string(),
            }],
        });
        response.status = (403, "Forbidden");
        return Some(response);
    }
}

from_error!(Unauthorized);

pub fn internal_error<S1: Str, S2: Str>(error: S1,
                                        detail: S2) -> Box<CargoError> {
    box ConcreteCargoError {
        description: error.as_slice().to_string(),
        detail: Some(detail.as_slice().to_string()),
        cause: None,
        human: false,
    } as Box<CargoError>
}

pub fn internal<S: Show>(error: S) -> Box<CargoError> {
    box ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: false,
    } as Box<CargoError>
}

pub fn human<S: Show>(error: S) -> Box<CargoError> {
    box ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: true,
    } as Box<CargoError>
}

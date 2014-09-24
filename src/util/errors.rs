use std::io::{IoError, MemReader};
use std::fmt;
use std::fmt::{Show, Formatter, FormatError};
use std::collections::HashMap;

use conduit::Response;
use curl::ErrCode;
use pg::error::{PostgresError, PostgresConnectError};
use serialize::json;
use git2;

use util::json_response;

pub trait CargoError: Send {
    fn description(&self) -> String;
    fn detail(&self) -> Option<String> { None }
    fn cause<'a>(&'a self) -> Option<&'a CargoError + Send> { None }

    fn to_error<E: FromError<Self>>(self) -> E {
        FromError::from_error(self)
    }

    fn box_error(self) -> Box<CargoError + Send> {
        box self as Box<CargoError + Send>
    }

    fn concrete(&self) -> ConcreteCargoError {
        ConcreteCargoError {
            description: self.description(),
            detail: self.detail(),
            cause: self.cause().map(|c| box c.concrete() as Box<CargoError + Send>),
            human: false,
        }
    }

    fn with_cause<E: CargoError + Send>(self, cause: E) -> Box<CargoError + Send> {
        let mut concrete = self.concrete();
        concrete.cause = Some(cause.box_error());
        box concrete as Box<CargoError + Send>
    }

    fn response(&self) -> Option<Response> {
        #[deriving(Encodable)] struct Bad { ok: bool, error: String }

        if self.human() {
            Some(json_response(&Bad {
                ok: false,
                error: self.description(),
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

impl<E: CargoError + Send> FromError<E> for Box<CargoError + Send> {
    fn from_error(error: E) -> Box<CargoError + Send> {
        error.box_error()
    }
}

macro_rules! from_error (
    ($ty:ty) => {
        impl FromError<$ty> for $ty {
            fn from_error(error: $ty) -> $ty {
                error
            }
        }
    }
)

impl<'a> Show for &'a CargoError + Send {
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

impl Show for Box<CargoError + Send> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let me: &CargoError + Send = &**self;
        me.fmt(f)
    }
}

impl CargoError for Box<CargoError + Send> {
    fn description(&self) -> String {
        (*self).description()
    }

    fn detail(&self) -> Option<String> {
        (*self).detail()
    }

    fn cause<'a>(&'a self) -> Option<&'a CargoError + Send> {
        (*self).cause()
    }

    fn box_error(self) -> Box<CargoError + Send> {
        self
    }
}

pub type CargoResult<T> = Result<T, Box<CargoError + Send>>;

pub trait BoxError<T> {
    fn box_error(self) -> CargoResult<T>;
}

pub trait ChainError<T> {
    fn chain_error<E: CargoError + Send>(self, callback: || -> E) -> CargoResult<T> ;
}

impl<'a, T> ChainError<T> for ||:'a -> CargoResult<T> {
    fn chain_error<E: CargoError + Send>(self, callback: || -> E) -> CargoResult<T> {
        self().map_err(|err| callback().with_cause(err))
    }
}

impl<T, E: CargoError + Send> BoxError<T> for Result<T, E> {
    fn box_error(self) -> CargoResult<T> {
        self.map_err(|err| err.box_error())
    }
}

impl<T, E: CargoError + Send> ChainError<T> for Result<T, E> {
    fn chain_error<E: CargoError + Send>(self, callback: || -> E) -> CargoResult<T>  {
        self.map_err(|err| callback().with_cause(err))
    }
}

impl CargoError for IoError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(IoError)

impl CargoError for FormatError {
    fn description(&self) -> String {
        "formatting failed".to_string()
    }
}

from_error!(FormatError)

impl CargoError for PostgresError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(PostgresError)

impl CargoError for PostgresConnectError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(PostgresConnectError)

impl CargoError for ErrCode {
    fn description(&self) -> String { self.to_string() }
}

from_error!(ErrCode)

impl CargoError for json::DecoderError {
    fn description(&self) -> String { self.to_string() }
}

from_error!(json::DecoderError)

impl CargoError for git2::Error {
    fn description(&self) -> String { self.to_string() }
}

from_error!(git2::Error)

impl<T: CargoError + Send> FromError<T> for Box<Show + 'static> {
    fn from_error(t: T) -> Box<Show + 'static> {
        box() (box t as Box<CargoError + Send>) as Box<Show + 'static>
    }
}

pub struct ConcreteCargoError {
    description: String,
    detail: Option<String>,
    cause: Option<Box<CargoError + Send>>,
    human: bool,
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

    fn cause<'a>(&'a self) -> Option<&'a CargoError + Send> {
        self.cause.as_ref().map(|c| { let err: &CargoError + Send = &**c; err })
    }

    fn with_cause<E: CargoError + Send>(mut self,
                                        err: E) -> Box<CargoError + Send> {
        self.cause = Some(err.box_error());
        box self as Box<CargoError + Send>
    }
    fn human(&self) -> bool { self.human }
}

pub struct NotFound;

impl CargoError for NotFound {
    fn description(&self) -> String { "not found".to_string() }

    fn response(&self) -> Option<Response> {
        Some(Response {
            status: (404, "Not Found"),
            headers: HashMap::new(),
            body: box MemReader::new(Vec::new()),
        })
    }
}

from_error!(NotFound)

pub struct Unauthorized;

impl CargoError for Unauthorized {
    fn description(&self) -> String { "unauthorized".to_string() }

    fn response(&self) -> Option<Response> {
        Some(Response {
            status: (403, "Forbidden"),
            headers: HashMap::new(),
            body: box MemReader::new(Vec::new()),
        })
    }
}

from_error!(Unauthorized)

pub fn internal_error<S1: Str, S2: Str>(error: S1,
                                        detail: S2) -> Box<CargoError + Send> {
    box ConcreteCargoError {
        description: error.as_slice().to_string(),
        detail: Some(detail.as_slice().to_string()),
        cause: None,
        human: false,
    } as Box<CargoError + Send>
}

pub fn internal<S: Show>(error: S) -> Box<CargoError + Send> {
    box ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: false,
    } as Box<CargoError + Send>
}

pub fn human<S: Show>(error: S) -> Box<CargoError + Send> {
    box ConcreteCargoError {
        description: error.to_string(),
        detail: None,
        cause: None,
        human: true,
    } as Box<CargoError + Send>
}

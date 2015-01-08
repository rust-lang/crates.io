use util::errors::{CargoResult, CargoError};

pub trait Wrap {
    fn wrap<E: CargoError>(self, error: E) -> Self;
}

impl<T> Wrap for Result<T, Box<CargoError>> {
    fn wrap<E: CargoError>(self, error: E) -> CargoResult<T> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(error.concrete().with_cause(e))
        }
    }
}

pub trait Require<T> {
    fn require<E: CargoError, F>(self, err: F) -> CargoResult<T>
        where F: FnOnce() -> E;
}

impl<T> Require<T> for Option<T> {
    fn require<E: CargoError, F>(self, err: F) -> CargoResult<T>
        where F: FnOnce() -> E
    {
        match self {
            Some(x) => Ok(x),
            None => Err(box err() as Box<CargoError>)
        }
    }
}

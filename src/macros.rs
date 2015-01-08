macro_rules! try{ ($expr:expr) => ({
    use util::errors::FromError;
    match $expr.map_err(FromError::from_error) {
        Ok(val) => val, Err(err) => return Err(err)
    }
}) }

macro_rules! raw_try{ ($expr:expr) => (
    match $expr { Ok(val) => val, Err(err) => return Err(err) }
) }

macro_rules! try_option{ ($e:expr) => (
    match $e { Some(k) => k, None => return None }
) }

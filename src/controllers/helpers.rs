use crate::util::{json_response, EndpointResult};

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> EndpointResult {
    let json = json!({ "ok": true });
    Ok(json_response(&json))
}

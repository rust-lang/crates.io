use crate::util::{json_response, EndpointResult};

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> EndpointResult {
    #[derive(Serialize)]
    struct R {
        ok: bool,
    }

    Ok(json_response(&R { ok: true }))
}

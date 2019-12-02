use crate::util::{json_response, AppResult};
use conduit::Response;

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> AppResult<Response> {
    #[derive(Serialize)]
    struct R {
        ok: bool,
    }

    Ok(json_response(&R { ok: true }))
}

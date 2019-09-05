use crate::util::{json_response, CargoResult};
use conduit::Response;

pub(crate) mod pagination;

pub(crate) use self::pagination::Paginate;

pub fn ok_true() -> CargoResult<Response> {
    #[derive(Serialize)]
    struct R {
        ok: bool,
    }

    Ok(json_response(&R { ok: true }))
}

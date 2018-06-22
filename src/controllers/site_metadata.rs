use super::prelude::*;

/// Returns the JSON representation of the current deployed commit sha.
///
/// The sha is contained within the `HEROKU_SLUG_COMMIT` environment variable.
/// If `HEROKU_SLUG_COMMIT` is not set, returns `"unknown"`.
pub fn show_deployed_sha(req: &mut dyn Request) -> CargoResult<Response> {
    let deployed_sha =
        ::std::env::var("HEROKU_SLUG_COMMIT").unwrap_or_else(|_| String::from("unknown"));

    #[derive(Serialize)]
    struct R {
        deployed_sha: String,
    }
    Ok(req.json(&R { deployed_sha }))
}

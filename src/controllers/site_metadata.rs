use super::prelude::*;

/// Returns the JSON representation of the current deployed commit sha.
///
/// The sha is contained within the `HEROKU_SLUG_COMMIT` environment variable.
/// If `HEROKU_SLUG_COMMIT` is not set, returns `"unknown"`.
pub fn show_deployed_sha(req: &mut dyn RequestExt) -> EndpointResult {
    let config = &req.app().config;
    let read_only = config.db.are_all_read_only();

    let deployed_sha =
        dotenv::var("HEROKU_SLUG_COMMIT").unwrap_or_else(|_| String::from("unknown"));

    Ok(req.json(&json!({
        "deployed_sha": &deployed_sha[..],
        "commit": &deployed_sha[..],
        "read_only": read_only,
    })))
}

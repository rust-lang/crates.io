use super::prelude::*;

/// Returns the JSON representation of the current deployed commit sha.
///
/// The sha is contained within the `HEROKU_SLUG_COMMIT` environment variable.
/// If `HEROKU_SLUG_COMMIT` is not set, returns `"unknown"`.
pub fn show_deployed_sha(req: &mut dyn RequestExt) -> EndpointResult {
    let deployed_sha =
        dotenv::var("HEROKU_SLUG_COMMIT").unwrap_or_else(|_| String::from("unknown"));

    Ok(req.json(&json!({
        "deployed_sha": &deployed_sha[..],
        "commit": &deployed_sha[..],
    })))
}

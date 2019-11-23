use super::prelude::*;

/// Returns the JSON representation of the current deployed commit sha.
///
/// The sha is contained within the `HEROKU_SLUG_COMMIT` environment variable.
/// If `HEROKU_SLUG_COMMIT` is not set, returns `"unknown"`.
pub fn show_deployed_sha(req: &mut dyn Request) -> AppResult<Response> {
    let deployed_sha =
        dotenv::var("HEROKU_SLUG_COMMIT").unwrap_or_else(|_| String::from("unknown"));

    #[derive(Serialize)]
    struct R<'a> {
        deployed_sha: &'a str,
        commit: &'a str,
    }
    Ok(req.json(&R {
        deployed_sha: &deployed_sha[..],
        commit: &deployed_sha[..],
    }))
}

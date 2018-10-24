use conduit::Request;

/// Returns the value of the request header, or an empty string if it is not
/// present.
///
/// The implementation should return only the first line if it's a multiline
/// header, but rust-civet is just wrapping the whole thing in a `vec!` anyway,
/// and I have no clue what the C code it's wrapping does.
///
/// The C library rust-civet is wrapping does not document whether headers are
/// case sensitive or not, so I have no clue if this follows the HTTP spec in
/// that regard.
pub fn request_header<'a>(req: &'a dyn Request, header_name: &str) -> &'a str {
    req.headers()
        .find(header_name)
        .and_then(|x| x.first().cloned())
        .unwrap_or_default()
}

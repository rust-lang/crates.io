use derive_more::{Deref, Display};
use serde::Deserialize;

/// A string that does not contain null bytes (`\0`).
#[derive(Clone, Debug, Display, Deref, Deserialize, utoipa::ToSchema)]
#[serde(try_from = "String")]
pub struct StringExclNull(String);

/// Error indicating that a string contained a null byte.
#[derive(Debug, thiserror::Error)]
#[error("string contains null byte")]
pub struct StringExclNullError;

impl TryFrom<String> for StringExclNull {
    type Error = StringExclNullError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.contains('\0') {
            Err(StringExclNullError)
        } else {
            Ok(Self(s))
        }
    }
}

pub mod cloudfront;
mod download_map;
pub mod fastly;
mod paths;
#[cfg(test)]
mod test_utils;

pub use crate::download_map::DownloadsMap;

#![crate_id = "html"]

extern crate url;

pub use escape::Escape;

mod escape;
pub mod form;

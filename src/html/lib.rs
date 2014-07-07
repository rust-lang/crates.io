#![crate_name = "html"]
#![feature(macro_rules)]

extern crate url;

pub use escape::Escape;

mod escape;
pub mod form;

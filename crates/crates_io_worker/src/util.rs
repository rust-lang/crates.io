use anyhow::anyhow;
use std::any::Any;
use std::panic::PanicHookInfo;

/// Tries to figure out what's in the box, and wraps it in an error if we can.
///
/// The actual error type we will get from `panic::catch_unwind` is really poorly documented.
/// However, the `panic::set_hook` functions deal with a `PanicHookInfo` type, and its payload is
/// documented as "commonly but not always `&'static str` or `String`". So we can try all of those,
/// and give up if we didn't get one of those three types.
pub fn try_to_extract_panic_info(info: &(dyn Any + Send + 'static)) -> anyhow::Error {
    if let Some(x) = info.downcast_ref::<PanicHookInfo<'_>>() {
        anyhow!("job panicked: {x}")
    } else if let Some(x) = info.downcast_ref::<&'static str>() {
        anyhow!("job panicked: {x}")
    } else if let Some(x) = info.downcast_ref::<String>() {
        anyhow!("job panicked: {x}")
    } else {
        anyhow!("job panicked")
    }
}

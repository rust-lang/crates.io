/// Access rights to the crate (publishing and ownership management)
/// NOTE: The order of these variants matters!
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum Rights {
    None,
    Publish,
    Full,
}

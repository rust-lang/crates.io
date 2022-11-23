#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointScope {
    PublishNew,
    PublishUpdate,
    Yank,
    ChangeOwners,
}

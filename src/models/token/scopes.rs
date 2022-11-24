use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, AsExpression)]
#[sql_type = "Text"]
pub enum EndpointScope {
    PublishNew,
    PublishUpdate,
    Yank,
    ChangeOwners,
}

impl ToSql<Text, Pg> for EndpointScope {
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, Pg>) -> serialize::Result {
        match *self {
            EndpointScope::PublishNew => out.write_all(b"publish-new")?,
            EndpointScope::PublishUpdate => out.write_all(b"publish-update")?,
            EndpointScope::Yank => out.write_all(b"yank")?,
            EndpointScope::ChangeOwners => out.write_all(b"change-owners")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for EndpointScope {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"publish-new" => Ok(EndpointScope::PublishNew),
            b"publish-update" => Ok(EndpointScope::PublishUpdate),
            b"yank" => Ok(EndpointScope::Yank),
            b"change-owners" => Ok(EndpointScope::ChangeOwners),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

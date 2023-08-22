use bigdecimal::{BigDecimal, ToPrimitive};
use diesel::{
    deserialize::FromSql,
    pg::Pg,
    sql_types::{Numeric, Record},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::schema::sql_types::SemverTriple;

/// The Rust representation of the `semver_triple` composite type.
///
/// Note that this implements `FromSql` but not `ToSql` as this is only used in a generated column
/// and therefore we should never have to insert a record that includes an instance of this.
/// Implementing `ToSql` is trivial, but therefore unnecessary.
#[derive(Debug, Clone, AsExpression)]
#[diesel(sql_type = SemverTriple)]
pub struct Triple {
    pub major: u64,
    pub minor: u64,
    pub teeny: u64,
}

impl FromSql<SemverTriple, Pg> for Triple {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        // We have to round trip this through BigDecimal because the fields of the composite time
        // are `numeric`, rather than a bounded integer type. (This is because PostgreSQL doesn't
        // support unsigned integer types, so we can't use `bigint unsigned`, so to replicate a
        // Rust u64 we have to use numeric. See #1846 for more detail.)
        let (major, minor, teeny): (BigDecimal, BigDecimal, BigDecimal) =
            FromSql::<Record<(Numeric, Numeric, Numeric)>, Pg>::from_sql(bytes)?;
        Ok(Triple {
            major: major.to_u64().ok_or(Error::OutOfBounds {
                component: Component::Major,
                value: major,
            })?,
            minor: minor.to_u64().ok_or(Error::OutOfBounds {
                component: Component::Minor,
                value: minor,
            })?,
            teeny: teeny.to_u64().ok_or(Error::OutOfBounds {
                component: Component::Teeny,
                value: teeny,
            })?,
        })
    }
}

// `Triple` is used in `models::Version`, which requires it to implement `Serialize` and
// `Deserialize`. Deriving those traits would result in this being serialised as a map, which feels
// wasteful, since the field names are basically just noise. We'll serialise as a tuple instead.

#[derive(Deserialize, Serialize)]
struct SerializedTriple(u64, u64, u64);

impl From<&Triple> for SerializedTriple {
    fn from(value: &Triple) -> Self {
        Self(value.major, value.minor, value.teeny)
    }
}

impl From<SerializedTriple> for Triple {
    fn from(value: SerializedTriple) -> Self {
        Self {
            major: value.0,
            minor: value.1,
            teeny: value.2,
        }
    }
}

impl<'de> Deserialize<'de> for Triple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SerializedTriple::deserialize(deserializer)?.into())
    }
}

impl Serialize for Triple {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerializedTriple::from(self).serialize(serializer)
    }
}

#[derive(Error, Debug)]
enum Error {
    #[error("out of bounds {component:?} version component: {value}")]
    OutOfBounds {
        component: Component,
        value: BigDecimal,
    },
}

#[derive(Debug)]
enum Component {
    Major,
    Minor,
    Teeny,
}

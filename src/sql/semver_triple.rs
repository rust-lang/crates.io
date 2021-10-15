use bigdecimal::BigDecimal;
use diesel::deserialize::{self, FromSqlRow};
use diesel::pg::Pg;
use diesel::row::Row;
use diesel::serialize::{self, Output, ToSql, WriteTuple};
use diesel::sql_types::{Numeric, Record};
use diesel::Queryable;
use std::io::Write;

type BigDecimalTuple = (BigDecimal, BigDecimal, BigDecimal);
type NumericTuple = (Numeric, Numeric, Numeric);

#[derive(Debug, Clone, Deserialize, Serialize, SqlType, AsExpression, QueryId)]
#[postgres(type_name = "semver_triple")]
#[sql_type = "SemverTriple"]
pub struct SemverTriple {
    pub major: BigDecimal,
    pub minor: BigDecimal,
    pub patch: BigDecimal,
}

impl SemverTriple {
    pub fn new<T: Into<BigDecimal>>(major: T, minor: T, patch: T) -> Self {
        Self {
            major: major.into(),
            minor: minor.into(),
            patch: patch.into(),
        }
    }

    pub fn from_semver(semver: &semver::Version) -> Self {
        Self::new(semver.major, semver.minor, semver.patch)
    }

    pub fn from_semver_no_prerelease(semver: &semver::Version) -> Option<Self> {
        match semver {
            semver if semver.pre.is_empty() => Some(SemverTriple::from_semver(semver)),
            _ => None,
        }
    }
}

impl ToSql<SemverTriple, Pg> for SemverTriple {
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, Pg>) -> serialize::Result {
        WriteTuple::<NumericTuple>::write_tuple(&(&self.major, &self.minor, &self.patch), out)
    }
}

impl Queryable<SemverTriple, Pg> for SemverTriple {
    type Row = <BigDecimalTuple as Queryable<Record<NumericTuple>, Pg>>::Row;

    fn build(row: Self::Row) -> Self {
        let (major, minor, patch) = row;
        Self::new(major, minor, patch)
    }
}

impl FromSqlRow<SemverTriple, Pg> for BigDecimalTuple {
    fn build_from_row<T: Row<Pg>>(row: &mut T) -> deserialize::Result<Self> {
        <BigDecimalTuple as FromSqlRow<Record<NumericTuple>, Pg>>::build_from_row(row)
    }
}

#[cfg(test)]
mod tests {
    use super::SemverTriple;
    use crate::test_util::pg_connection;
    use diesel::dsl::sql;
    use diesel::sql_types::Bool;
    use diesel::{select, RunQueryDsl};

    #[test]
    fn to_sql_works() {
        let conn = pg_connection();

        let sql = sql::<Bool>("(1, 2, 42)::semver_triple = ")
            .bind::<SemverTriple, _>(SemverTriple::new(1, 2, 42));

        let res = select(sql).get_result::<bool>(&conn);
        assert_ok_eq!(res, true);
    }
}

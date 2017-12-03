use diesel::pg::Pg;
use diesel::query_source::QueryableByName;
use diesel::row::NamedRow;
use std::error::Error;

pub struct WithCount<T> {
    total: i64,
    record: T,
}

impl<T> QueryableByName<Pg> for WithCount<T>
where
    T: QueryableByName<Pg>,
{
    fn build<R: NamedRow<Pg>>(row: &R) -> Result<Self, Box<Error + Send + Sync>> {
        use diesel::types::BigInt;

        Ok(WithCount {
            total: row.get::<BigInt, _>("total")?,
            record: T::build(row)?,
        })
    }
}

pub trait WithCountExtension<T> {
    fn records_and_total(self) -> (Vec<T>, i64);
}

impl<T> WithCountExtension<T> for Vec<WithCount<T>> {
    fn records_and_total(self) -> (Vec<T>, i64) {
        let cnt = self.get(0).map(|row| row.total).unwrap_or(0);
        let vec = self.into_iter().map(|row| row.record).collect();
        (vec, cnt)
    }
}

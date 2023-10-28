use diesel::sql_types::BigInt;

#[derive(QueryableByName, Queryable, Debug)]
pub struct WithCount<T> {
    #[diesel(embed)]
    pub(crate) record: T,
    #[diesel(sql_type = BigInt)]
    pub(crate) total: i64,
}

pub trait WithCountExtension<T> {
    fn records_and_total(self) -> (Vec<T>, i64);
}

impl<T> WithCountExtension<T> for Vec<WithCount<T>> {
    fn records_and_total(self) -> (Vec<T>, i64) {
        let cnt = self.first().map(|row| row.total).unwrap_or(0);
        let vec = self.into_iter().map(|row| row.record).collect();
        (vec, cnt)
    }
}

#[derive(QueryableByName, Debug)]
pub struct WithCount<T> {
    #[sql_type = "::diesel::sql_types::BigInt"]
    total: i64,
    #[diesel(embed)]
    record: T,
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

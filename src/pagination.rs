use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::types::BigInt;
use diesel::pg::Pg;

pub struct Paginated<T> {
    query: T,
    limit: i64,
    offset: i64,
}

pub trait Paginate: AsQuery + Sized {
    fn paginate(self, limit: i64, offset: i64) -> Paginated<Self::Query> {
        Paginated {
            query: self.as_query(),
            limit,
            offset,
        }
    }
}

impl<T: AsQuery> Paginate for T {}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.limit)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}

impl_query_id!(Paginated<T>);

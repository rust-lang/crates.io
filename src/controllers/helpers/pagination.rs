use crate::models::helpers::with_count::*;
use crate::util::errors::{bad_request, AppResult};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::LoadQuery;
use diesel::sql_types::BigInt;
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Page {
    Numeric(u32),
    Unspecified,
}

impl Page {
    fn new(params: &IndexMap<String, String>) -> AppResult<Self> {
        if let Some(s) = params.get("page") {
            let numeric_page = s.parse().map_err(|e| bad_request(&e))?;
            if numeric_page < 1 {
                return Err(bad_request(&format_args!(
                    "page indexing starts from 1, page {} is invalid",
                    numeric_page,
                )));
            }

            Ok(Page::Numeric(numeric_page))
        } else {
            Ok(Page::Unspecified)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PaginationOptions {
    page: Page,
    pub(crate) per_page: u32,
}

impl PaginationOptions {
    pub(crate) fn new(params: &IndexMap<String, String>) -> AppResult<Self> {
        const DEFAULT_PER_PAGE: u32 = 10;
        const MAX_PER_PAGE: u32 = 100;

        let per_page = params
            .get("per_page")
            .map(|s| s.parse().map_err(|e| bad_request(&e)))
            .unwrap_or(Ok(DEFAULT_PER_PAGE))?;

        if per_page > MAX_PER_PAGE {
            return Err(bad_request(&format_args!(
                "cannot request more than {} items",
                MAX_PER_PAGE,
            )));
        }

        Ok(Self {
            page: Page::new(params)?,
            per_page,
        })
    }

    pub(crate) fn offset(&self) -> Option<u32> {
        if let Page::Numeric(p) = self.page {
            Some((p - 1) * self.per_page)
        } else {
            None
        }
    }
}

pub(crate) trait Paginate: Sized {
    fn paginate(self, params: &IndexMap<String, String>) -> AppResult<PaginatedQuery<Self>> {
        Ok(PaginatedQuery {
            query: self,
            options: PaginationOptions::new(params)?,
        })
    }
}

impl<T> Paginate for T {}

pub struct Paginated<T> {
    records_and_total: Vec<WithCount<T>>,
    options: PaginationOptions,
}

impl<T> Paginated<T> {
    pub(crate) fn total(&self) -> Option<i64> {
        Some(
            self.records_and_total
                .get(0)
                .map(|row| row.total)
                .unwrap_or_default(),
        )
    }

    pub(crate) fn next_page_params(&self) -> Option<IndexMap<String, String>> {
        if self.records_and_total.len() < self.options.per_page as usize {
            return None;
        }

        let mut opts = IndexMap::new();
        match self.options.page {
            Page::Numeric(n) => opts.insert("page".into(), (n + 1).to_string()),
            Page::Unspecified => opts.insert("page".into(), 2.to_string()),
        };
        Some(opts)
    }

    pub(crate) fn prev_page_params(&self) -> Option<IndexMap<String, String>> {
        if let Page::Numeric(1) | Page::Unspecified = self.options.page {
            return None;
        }

        let mut opts = IndexMap::new();
        match self.options.page {
            Page::Numeric(n) => opts.insert("page".into(), (n - 1).to_string()),
            Page::Unspecified => unreachable!(),
        };
        Some(opts)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.records_and_total.iter().map(|row| &row.record)
    }
}

impl<T: 'static> IntoIterator for Paginated<T> {
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.records_and_total.into_iter().map(|row| row.record))
    }
}

#[derive(Debug)]
pub(crate) struct PaginatedQuery<T> {
    query: T,
    options: PaginationOptions,
}

impl<T> PaginatedQuery<T> {
    pub(crate) fn load<U>(self, conn: &PgConnection) -> QueryResult<Paginated<U>>
    where
        Self: LoadQuery<PgConnection, WithCount<U>>,
    {
        let options = self.options;
        let records_and_total = self.internal_load(conn)?;
        Ok(Paginated {
            records_and_total,
            options,
        })
    }
}

impl<T> QueryId for PaginatedQuery<T> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<T: Query> Query for PaginatedQuery<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T, DB> RunQueryDsl<DB> for PaginatedQuery<T> {}

impl<T> QueryFragment<Pg> for PaginatedQuery<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<'_, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&i64::from(self.options.per_page))?;
        if let Some(offset) = self.options.offset() {
            out.push_sql(" OFFSET ");
            out.push_bind_param::<BigInt, _>(&i64::from(offset))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Page, PaginationOptions};
    use indexmap::IndexMap;

    #[test]
    fn page_must_be_a_number() {
        let mut params = IndexMap::new();
        params.insert(String::from("page"), String::from("not a number"));
        let page_error = Page::new(&params).unwrap_err().response().unwrap();

        assert_eq!(page_error.status, (400, "Bad Request"));
    }

    #[test]
    fn per_page_must_be_a_number() {
        let mut params = IndexMap::new();
        params.insert(String::from("per_page"), String::from("not a number"));
        let per_page_error = PaginationOptions::new(&params)
            .unwrap_err()
            .response()
            .unwrap();

        assert_eq!(per_page_error.status, (400, "Bad Request"));
    }
}

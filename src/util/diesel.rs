use diesel::connection::LoadConnection;
use diesel::pg::Pg;
use diesel::result::Error;

pub trait Conn: LoadConnection<Backend = Pg> {}

impl<T> Conn for T where T: LoadConnection<Backend = Pg> {}

pub fn is_read_only_error(error: &Error) -> bool {
    matches!(error, Error::DatabaseError(_, info) if info.message().ends_with("read-only transaction"))
}

pub mod prelude {
    //! Inline diesel prelude
    pub use diesel::associations::{Associations, GroupedBy, Identifiable};
    pub use diesel::connection::Connection;
    pub use diesel::deserialize::{Queryable, QueryableByName};
    pub use diesel::expression::IntoSql as _;
    pub use diesel::expression::{
        AppearsOnTable, BoxableExpression, Expression, IntoSql, Selectable, SelectableExpression,
    };

    pub use diesel::expression::functions::define_sql_function;

    pub use diesel::expression::SelectableHelper;
    pub use diesel::expression_methods::*;
    pub use diesel::insertable::Insertable;
    pub use diesel::prelude::{
        allow_columns_to_appear_in_same_group_by_clause, allow_tables_to_appear_in_same_query,
        joinable, table,
    };
    pub use diesel::query_builder::AsChangeset;
    pub use diesel::query_builder::DecoratableTarget;
    pub use diesel::query_dsl::{BelongingToDsl, CombineDsl, JoinOnDsl, QueryDsl, SaveChangesDsl};
    pub use diesel::query_source::SizeRestrictedColumn as _;
    pub use diesel::query_source::{Column, JoinTo, QuerySource, Table};
    pub use diesel::result::{
        ConnectionError, ConnectionResult, OptionalEmptyChangesetExtension, OptionalExtension,
        QueryResult,
    };

    pub use diesel::prelude::ExecuteCopyFromDsl;
    pub use diesel::prelude::PgConnection;
}

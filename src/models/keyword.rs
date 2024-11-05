use chrono::NaiveDateTime;
use diesel_async::AsyncPgConnection;

use crate::models::Crate;
use crate::schema::*;
use crate::sql::lower;
use crate::util::diesel::prelude::*;
use crate::util::diesel::Conn;

#[derive(Clone, Identifiable, Queryable, Debug, Selectable)]
pub struct Keyword {
    pub id: i32,
    pub keyword: String,
    pub crates_cnt: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Associations, Insertable, Identifiable, Debug, Clone, Copy)]
#[diesel(
    table_name = crates_keywords,
    check_for_backend(diesel::pg::Pg),
    primary_key(crate_id, keyword_id),
    belongs_to(Keyword),
    belongs_to(Crate),
)]
pub struct CrateKeyword {
    crate_id: i32,
    keyword_id: i32,
}

impl Keyword {
    pub async fn find_by_keyword(conn: &mut AsyncPgConnection, name: &str) -> QueryResult<Keyword> {
        use diesel_async::RunQueryDsl;

        keywords::table
            .filter(keywords::keyword.eq(lower(name)))
            .first(conn)
            .await
    }

    pub fn find_or_create_all(conn: &mut impl Conn, names: &[&str]) -> QueryResult<Vec<Keyword>> {
        use diesel::RunQueryDsl;

        let lowercase_names: Vec<_> = names.iter().map(|s| s.to_lowercase()).collect();

        let new_keywords: Vec<_> = lowercase_names
            .iter()
            .map(|s| keywords::keyword.eq(s))
            .collect();

        diesel::insert_into(keywords::table)
            .values(&new_keywords)
            .on_conflict_do_nothing()
            .execute(conn)?;
        keywords::table
            .filter(keywords::keyword.eq_any(&lowercase_names))
            .load(conn)
    }

    pub fn valid_name(name: &str) -> bool {
        let mut chars = name.chars();
        let first = match chars.next() {
            None => return false,
            Some(c) => c,
        };
        first.is_ascii_alphanumeric()
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '+')
    }

    pub fn update_crate(conn: &mut impl Conn, crate_id: i32, keywords: &[&str]) -> QueryResult<()> {
        conn.transaction(|conn| {
            use diesel::RunQueryDsl;

            let keywords = Keyword::find_or_create_all(conn, keywords)?;

            diesel::delete(crates_keywords::table)
                .filter(crates_keywords::crate_id.eq(crate_id))
                .execute(conn)?;

            let crate_keywords = keywords
                .into_iter()
                .map(|kw| CrateKeyword {
                    crate_id,
                    keyword_id: kw.id,
                })
                .collect::<Vec<_>>();

            diesel::insert_into(crates_keywords::table)
                .values(&crate_keywords)
                .execute(conn)?;

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_db_connection;

    #[test]
    fn dont_associate_with_non_lowercased_keywords() {
        use diesel::RunQueryDsl;

        let (_test_db, conn) = &mut test_db_connection();
        // The code should be preventing lowercased keywords from existing,
        // but if one happens to sneak in there, don't associate crates with it.

        diesel::insert_into(keywords::table)
            .values(keywords::keyword.eq("NO"))
            .execute(conn)
            .unwrap();

        let associated = Keyword::find_or_create_all(conn, &["no"]).unwrap();
        assert_eq!(associated.len(), 1);
        assert_eq!(associated.first().unwrap().keyword, "no");
    }
}

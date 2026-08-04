/// Replaces all instances of `-` with `_` in the given username
pub fn canon_username(username: &str) -> String {
    username.replace("-", "_").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_database::fns::canon_username as canon_username_sql;
    use crates_io_test_db::TestDatabase;
    use diesel_async::RunQueryDsl;

    const USERNAMES: &[(&str, &str)] = &[
        ("foo", "foo"),
        ("Foo", "foo"),
        ("FOO", "foo"),
        ("foo-bar", "foo_bar"),
        ("foo_bar", "foo_bar"),
        ("Foo-Bar", "foo_bar"),
        ("FOO-BAR", "foo_bar"),
        ("foo-biz-bar", "foo_biz_bar"),
        ("foo--bar", "foo__bar"),
        ("-foo-", "_foo_"),
        ("-", "_"),
        ("user-2", "user_2"),
        ("github:User-2", "github:user_2"),
        ("", ""),
    ];

    #[test]
    fn normalizes_case_and_separators() {
        for &(input, expected) in USERNAMES {
            assert_eq!(canon_username(input), expected, "canon_username({input:?})");
        }
    }

    #[test]
    fn usernames_differing_only_by_case_or_separator_match() {
        assert_eq!(canon_username("foo-bar"), canon_username("foo_bar"));
        assert_eq!(canon_username("Foo-Bar"), canon_username("fOO_bAR"));
        assert_eq!(canon_username("user-2"), canon_username("USER_2"));
    }

    #[test]
    fn distinct_usernames_do_not_match() {
        assert_ne!(canon_username("foobar"), canon_username("foo_bar"));
        assert_ne!(canon_username("foo-bar"), canon_username("foo--bar"));
        assert_ne!(canon_username("alice"), canon_username("alice2"));
    }

    #[tokio::test]
    async fn matches_the_canon_username_sql_implementation() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        for &(input, _) in USERNAMES {
            let from_sql: String = diesel::select(canon_username_sql(input))
                .get_result(&mut conn)
                .await
                .unwrap();

            assert_eq!(canon_username(input), from_sql, "canon_username({input:?})");
        }
    }
}

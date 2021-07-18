use crate::schema::{publish_limit_buckets, publish_rate_overrides};
use crate::util::errors::{AppResult, TooManyRequests};
use chrono::{NaiveDateTime, Utc};
use diesel::data_types::PgInterval;
use diesel::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

crate::pg_enum! {
    pub enum LimitedAction {
        PublishNew = 0,
    }
}

impl LimitedAction {
    /// How many seconds should elapse between requests (after the burst is exhausted).
    pub fn default_rate_seconds(&self) -> u64 {
        match self {
            LimitedAction::PublishNew => 60 * 10,
        }
    }

    /// How many requests a user can make before the rate limit goes into effect.
    pub fn default_burst(&self) -> i32 {
        match self {
            LimitedAction::PublishNew => 5,
        }
    }

    /// Key used to identify this action in environment variables. See `src/config.rs`.
    pub fn env_var_key(&self) -> &'static str {
        match self {
            LimitedAction::PublishNew => "PUBLISH_NEW",
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RateLimiterConfig {
    pub rate: Duration,
    pub burst: i32,
}

#[derive(Debug)]
pub struct RateLimiter {
    config: HashMap<LimitedAction, RateLimiterConfig>,
}

impl RateLimiter {
    pub fn new(config: HashMap<LimitedAction, RateLimiterConfig>) -> Self {
        Self { config }
    }

    pub fn check_rate_limit(
        &self,
        user_id: i32,
        action: LimitedAction,
        conn: &PgConnection,
    ) -> AppResult<()> {
        let bucket = self.take_token(user_id, action, Utc::now().naive_utc(), conn)?;
        if bucket.tokens >= 1 {
            Ok(())
        } else {
            Err(Box::new(TooManyRequests {
                retry_after: bucket.last_refill
                    + chrono::Duration::from_std(self.config[&action].rate).unwrap(),
            }))
        }
    }

    /// Refill a user's bucket as needed, take a token from it,
    /// and returns the result.
    ///
    /// The number of tokens remaining will always be between 0 and self.burst.
    /// If the number is 0, the request should be rejected, as the user doesn't
    /// have a token to take. Technically a "full" bucket would have
    /// `self.burst + 1` tokens in it, but that value would never be returned
    /// since we only refill buckets when trying to take a token from it.
    fn take_token(
        &self,
        user_id: i32,
        action: LimitedAction,
        now: NaiveDateTime,
        conn: &PgConnection,
    ) -> QueryResult<Bucket> {
        use diesel::sql_types::{Double, Interval, Text, Timestamp};

        sql_function!(fn date_part(x: Text, y: Timestamp) -> Double);
        sql_function! {
            #[sql_name = "date_part"]
            fn interval_part(x: Text, y: Interval) -> Double;
        }
        sql_function!(fn floor(x: Double) -> Integer);
        sql_function!(fn greatest<T>(x: T, y: T) -> T);
        sql_function!(fn least<T>(x: T, y: T) -> T);

        let burst: i32 = publish_rate_overrides::table
            .find((user_id, action))
            .filter(
                publish_rate_overrides::expires_at
                    .is_null()
                    .or(publish_rate_overrides::expires_at.gt(now)),
            )
            .select(publish_rate_overrides::burst)
            .first(conn)
            .optional()?
            .unwrap_or(self.config[&action].burst);

        // Interval division is poorly defined in general (what is 1 month / 30 days?)
        // However, for the intervals we're dealing with, it is always well
        // defined, so we convert to an f64 of seconds to represent this.
        let tokens_to_add = floor(
            (date_part("epoch", now) - date_part("epoch", publish_limit_buckets::last_refill))
                / interval_part("epoch", self.refill_rate(action)),
        );

        diesel::insert_into(publish_limit_buckets::table)
            .values((
                publish_limit_buckets::user_id.eq(user_id),
                publish_limit_buckets::action.eq(action),
                publish_limit_buckets::tokens.eq(burst),
                publish_limit_buckets::last_refill.eq(now),
            ))
            .on_conflict((
                publish_limit_buckets::user_id,
                publish_limit_buckets::action,
            ))
            .do_update()
            .set((
                publish_limit_buckets::tokens.eq(least(
                    burst,
                    greatest(0, publish_limit_buckets::tokens - 1) + tokens_to_add,
                )),
                publish_limit_buckets::last_refill.eq(publish_limit_buckets::last_refill
                    + self.refill_rate(action).into_sql::<Interval>() * tokens_to_add),
            ))
            .get_result(conn)
    }

    fn refill_rate(&self, action: LimitedAction) -> PgInterval {
        use diesel::dsl::*;
        (self.config[&action].rate.as_millis() as i64).milliseconds()
    }
}

#[derive(Queryable, Insertable, Debug, PartialEq, Clone, Copy)]
#[table_name = "publish_limit_buckets"]
#[allow(dead_code)] // Most fields only read in tests
struct Bucket {
    user_id: i32,
    tokens: i32,
    last_refill: NaiveDateTime,
    action: LimitedAction,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::Emails;
    use crate::test_util::*;

    #[test]
    fn take_token_with_no_bucket_creates_new_one() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let bucket = rate.take_token(
            new_user(&conn, "user1")?,
            LimitedAction::PublishNew,
            now,
            &conn,
        )?;
        let expected = Bucket {
            user_id: bucket.user_id,
            tokens: 10,
            last_refill: now,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);

        let rate = simple_limiter(LimitedAction::PublishNew, 50, 20);
        let bucket = rate.take_token(
            new_user(&conn, "user2")?,
            LimitedAction::PublishNew,
            now,
            &conn,
        )?;
        let expected = Bucket {
            user_id: bucket.user_id,
            tokens: 20,
            last_refill: now,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn take_token_with_existing_bucket_modifies_existing_bucket() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 5, now)?.user_id;
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, &conn)?;
        let expected = Bucket {
            user_id,
            tokens: 4,
            last_refill: now,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn take_token_after_delay_refills() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 5, now)?.user_id;
        let refill_time = now + chrono::Duration::seconds(2);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, &conn)?;
        let expected = Bucket {
            user_id,
            tokens: 6,
            last_refill: refill_time,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn refill_subsecond_rate() -> QueryResult<()> {
        let conn = pg_connection();
        // Subsecond rates have floating point rounding issues, so use a known
        // timestamp that rounds fine
        let now =
            NaiveDateTime::parse_from_str("2019-03-19T21:11:24.620401", "%Y-%m-%dT%H:%M:%S%.f")
                .unwrap();

        let rate = simple_limiter(LimitedAction::PublishNew, 100, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 5, now)?.user_id;
        let refill_time = now + chrono::Duration::milliseconds(300);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, &conn)?;
        let expected = Bucket {
            user_id,
            tokens: 7,
            last_refill: refill_time,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn last_refill_always_advanced_by_multiple_of_rate() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 100, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 5, now)?.user_id;
        let bucket = rate.take_token(
            user_id,
            LimitedAction::PublishNew,
            now + chrono::Duration::milliseconds(250),
            &conn,
        )?;
        let expected_refill_time = now + chrono::Duration::milliseconds(200);
        let expected = Bucket {
            user_id,
            tokens: 6,
            last_refill: expected_refill_time,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn zero_tokens_returned_when_user_has_no_tokens_left() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 1, now)?.user_id;
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, &conn)?;
        let expected = Bucket {
            user_id,
            tokens: 0,
            last_refill: now,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, &conn)?;
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn a_user_with_no_tokens_gets_a_token_after_exactly_rate() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 0, now)?.user_id;
        let refill_time = now + chrono::Duration::seconds(1);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, &conn)?;
        let expected = Bucket {
            user_id,
            tokens: 1,
            last_refill: refill_time,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);

        Ok(())
    }

    #[test]
    fn tokens_never_refill_past_burst() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user_bucket(&conn, LimitedAction::PublishNew, 8, now)?.user_id;
        let refill_time = now + chrono::Duration::seconds(4);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, &conn)?;
        let expected = Bucket {
            user_id,
            tokens: 10,
            last_refill: refill_time,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);

        Ok(())
    }

    #[test]
    fn override_is_used_instead_of_global_burst_if_present() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user(&conn, "user1")?;
        let other_user_id = new_user(&conn, "user2")?;

        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(user_id),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
                publish_rate_overrides::burst.eq(20),
            ))
            .execute(&conn)?;

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, &conn)?;
        let other_bucket = rate.take_token(other_user_id, LimitedAction::PublishNew, now, &conn)?;

        assert_eq!(20, bucket.tokens);
        assert_eq!(10, other_bucket.tokens);
        Ok(())
    }

    #[test]
    fn overrides_can_expire() -> QueryResult<()> {
        let conn = pg_connection();
        let now = now();

        let rate = simple_limiter(LimitedAction::PublishNew, 1000, 10);
        let user_id = new_user(&conn, "user1")?;
        let other_user_id = new_user(&conn, "user2")?;

        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(user_id),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
                publish_rate_overrides::burst.eq(20),
                publish_rate_overrides::expires_at.eq(now + chrono::Duration::days(30)),
            ))
            .execute(&conn)?;

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, &conn)?;
        let other_bucket = rate.take_token(other_user_id, LimitedAction::PublishNew, now, &conn)?;

        assert_eq!(20, bucket.tokens);
        assert_eq!(10, other_bucket.tokens);

        // Manually expire the rate limit
        diesel::update(publish_rate_overrides::table)
            .set(publish_rate_overrides::expires_at.eq(now - chrono::Duration::days(30)))
            .filter(publish_rate_overrides::user_id.eq(user_id))
            .execute(&conn)?;

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, &conn)?;
        let other_bucket = rate.take_token(other_user_id, LimitedAction::PublishNew, now, &conn)?;

        // The number of tokens of user_id is 10 and not 9 because when the new burst limit is
        // lower than the amount of available tokens, the number of available tokens is reset to
        // the new burst limit.
        assert_eq!(10, bucket.tokens);
        assert_eq!(9, other_bucket.tokens);

        Ok(())
    }

    fn new_user(conn: &PgConnection, gh_login: &str) -> QueryResult<i32> {
        use crate::models::NewUser;

        let user = NewUser {
            gh_login,
            ..NewUser::default()
        }
        .create_or_update(None, &Emails::new_in_memory(), conn)?;
        Ok(user.id)
    }

    fn new_user_bucket(
        conn: &PgConnection,
        action: LimitedAction,
        tokens: i32,
        now: NaiveDateTime,
    ) -> QueryResult<Bucket> {
        diesel::insert_into(publish_limit_buckets::table)
            .values(Bucket {
                user_id: new_user(conn, "new_user")?,
                tokens,
                last_refill: now,
                action,
            })
            .get_result(conn)
    }

    fn simple_limiter(action: LimitedAction, rate_millis: u64, burst: i32) -> RateLimiter {
        let mut config = HashMap::new();
        config.insert(
            action,
            RateLimiterConfig {
                rate: Duration::from_millis(rate_millis),
                burst,
            },
        );
        RateLimiter::new(config)
    }

    /// Strips ns precision from `Utc::now`. PostgreSQL only has microsecond
    /// precision, but some platforms (notably Linux) provide nanosecond
    /// precision, meaning that round tripping through the database would
    /// change the value.
    fn now() -> NaiveDateTime {
        let now = Utc::now().naive_utc();
        let nanos = now.timestamp_subsec_nanos();
        now - chrono::Duration::nanoseconds(nanos.into())
    }
}

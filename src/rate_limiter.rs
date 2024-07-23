use crate::schema::{publish_limit_buckets, publish_rate_overrides};
use crate::sql::{date_part, floor, greatest, interval_part, least, pg_enum};
use crate::util::diesel::Conn;
use crate::util::errors::{AppResult, TooManyRequests};
use chrono::{NaiveDateTime, Utc};
use diesel::dsl::IntervalDsl;
use diesel::prelude::*;
use diesel::sql_types::Interval;
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

pg_enum! {
    pub enum LimitedAction {
        PublishNew = 0,
        PublishUpdate = 1,
        YankUnyank = 2,
    }
}

impl LimitedAction {
    pub fn default_rate_seconds(&self) -> u64 {
        match self {
            LimitedAction::PublishNew => 10 * 60, // 10 minutes
            LimitedAction::PublishUpdate => 60,   // 1 minute
            LimitedAction::YankUnyank => 60,      // 1 minute
        }
    }

    pub fn default_burst(&self) -> i32 {
        match self {
            LimitedAction::PublishNew => 5,
            LimitedAction::PublishUpdate => 30,
            LimitedAction::YankUnyank => 100,
        }
    }

    pub fn env_var_key(&self) -> &'static str {
        match self {
            LimitedAction::PublishNew => "PUBLISH_NEW",
            LimitedAction::PublishUpdate => "PUBLISH_UPDATE",
            LimitedAction::YankUnyank => "YANK_UNYANK",
        }
    }

    pub fn error_message(&self) -> &'static str {
        match self {
            LimitedAction::PublishNew => {
                "You have published too many new crates in a short period of time"
            }
            LimitedAction::PublishUpdate => {
                "You have published too many updates to existing crates in a short period of time"
            }
            LimitedAction::YankUnyank => {
                "You have yanked or unyanked too many versions in a short period of time"
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
        uploader: i32,
        performed_action: LimitedAction,
        conn: &mut impl Conn,
    ) -> AppResult<()> {
        let bucket = self.take_token(uploader, performed_action, Utc::now().naive_utc(), conn)?;
        if bucket.tokens >= 1 {
            Ok(())
        } else {
            Err(Box::new(TooManyRequests {
                action: performed_action,
                retry_after: bucket.last_refill
                    + chrono::Duration::from_std(self.config_for_action(performed_action).rate)
                        .unwrap(),
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
        uploader: i32,
        performed_action: LimitedAction,
        now: NaiveDateTime,
        conn: &mut impl Conn,
    ) -> QueryResult<Bucket> {
        let config = self.config_for_action(performed_action);
        let refill_rate = (config.rate.as_millis() as i64).milliseconds();

        let burst: i32 = publish_rate_overrides::table
            .find((uploader, performed_action))
            .filter(
                publish_rate_overrides::expires_at
                    .is_null()
                    .or(publish_rate_overrides::expires_at.gt(now)),
            )
            .select(publish_rate_overrides::burst)
            .first(conn)
            .optional()?
            .unwrap_or(config.burst);

        // Interval division is poorly defined in general (what is 1 month / 30 days?)
        // However, for the intervals we're dealing with, it is always well
        // defined, so we convert to an f64 of seconds to represent this.
        let tokens_to_add = floor(
            (date_part("epoch", now) - date_part("epoch", publish_limit_buckets::last_refill))
                / interval_part("epoch", refill_rate),
        );

        diesel::insert_into(publish_limit_buckets::table)
            .values((
                publish_limit_buckets::user_id.eq(uploader),
                publish_limit_buckets::action.eq(performed_action),
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
                    + refill_rate.into_sql::<Interval>() * tokens_to_add),
            ))
            .get_result(conn)
    }

    fn config_for_action(&self, action: LimitedAction) -> Cow<'_, RateLimiterConfig> {
        // The wrapper returns the default config for the action when not configured.
        match self.config.get(&action) {
            Some(config) => Cow::Borrowed(config),
            None => Cow::Owned(RateLimiterConfig {
                rate: Duration::from_secs(action.default_rate_seconds()),
                burst: action.default_burst(),
            }),
        }
    }
}

#[derive(Queryable, Insertable, Debug, PartialEq, Clone, Copy)]
#[diesel(table_name = publish_limit_buckets, check_for_backend(diesel::pg::Pg))]
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
    fn default_rate_limits() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        // Set the defaults as if no env vars have been set in production
        let mut rate_limiter = HashMap::new();
        for action in LimitedAction::VARIANTS {
            rate_limiter.insert(
                *action,
                RateLimiterConfig {
                    rate: Duration::from_secs(action.default_rate_seconds()),
                    burst: action.default_burst(),
                },
            );
        }
        let rate = RateLimiter::new(rate_limiter);

        let user_id = new_user_bucket(conn, 5, now)?.user_id;

        // Publishing new crates has a burst of 5 and refill time of 1 every 10 minutes, which
        // means we should be able to publish every 10 min, always have tokens remaining, and
        // set the last_refill based on the refill time.
        let action = LimitedAction::PublishNew;
        let mut last_refill_times = vec![];
        let mut expected_last_refill_times = vec![];
        for publish_num in 1..=10 {
            let publish_time = now + chrono::Duration::minutes(10 * publish_num);
            let bucket = rate.take_token(user_id, action, publish_time, conn)?;

            last_refill_times.push(bucket.last_refill);
            expected_last_refill_times.push(publish_time);
        }
        assert_eq!(expected_last_refill_times, last_refill_times);

        // Publishing new versions has a burst of 30 and refill time of every minute, which
        // means we should be able to publish every min, always have tokens remaining, and
        // set the last_refill based on the refill time.
        let action = LimitedAction::PublishUpdate;
        let mut last_refill_times = vec![];
        let mut expected_last_refill_times = vec![];
        for publish_num in 1..=35 {
            let publish_time = now + chrono::Duration::minutes(publish_num);
            let bucket = rate.take_token(user_id, action, publish_time, conn)?;

            last_refill_times.push(bucket.last_refill);
            expected_last_refill_times.push(publish_time);
        }
        assert_eq!(expected_last_refill_times, last_refill_times);

        // Yanking/unyanking has a burst of 100 and refill time of every minute, which
        // means we should be able to yank/unyank every min, always have tokens remaining, and
        // set the last_refill based on the refill time.
        let action = LimitedAction::YankUnyank;
        let mut last_refill_times = vec![];
        let mut expected_last_refill_times = vec![];
        for publish_num in 1..=110 {
            let publish_time = now + chrono::Duration::minutes(publish_num);
            let bucket = rate.take_token(user_id, action, publish_time, conn)?;

            last_refill_times.push(bucket.last_refill);
            expected_last_refill_times.push(publish_time);
        }
        assert_eq!(expected_last_refill_times, last_refill_times);

        Ok(())
    }

    #[test]
    fn take_token_with_no_bucket_creates_new_one() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let bucket = rate.take_token(
            new_user(conn, "user1")?,
            LimitedAction::PublishNew,
            now,
            conn,
        )?;
        let expected = Bucket {
            user_id: bucket.user_id,
            tokens: 10,
            last_refill: now,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);

        let rate = SampleRateLimiter {
            rate: Duration::from_millis(50),
            burst: 20,
            action: LimitedAction::PublishNew,
        }
        .create();
        let bucket = rate.take_token(
            new_user(conn, "user2")?,
            LimitedAction::PublishNew,
            now,
            conn,
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
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 5, now)?.user_id;
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?;
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
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 5, now)?.user_id;
        let refill_time = now + chrono::Duration::seconds(2);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, conn)?;
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
        let (_test_db, conn) = &mut test_db_connection();
        // Subsecond rates have floating point rounding issues, so use a known
        // timestamp that rounds fine
        let now =
            NaiveDateTime::parse_from_str("2019-03-19T21:11:24.620401", "%Y-%m-%dT%H:%M:%S%.f")
                .unwrap();

        let rate = SampleRateLimiter {
            rate: Duration::from_millis(100),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 5, now)?.user_id;
        let refill_time = now + chrono::Duration::milliseconds(300);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, conn)?;
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
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_millis(100),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 5, now)?.user_id;
        let bucket = rate.take_token(
            user_id,
            LimitedAction::PublishNew,
            now + chrono::Duration::milliseconds(250),
            conn,
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
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 1, now)?.user_id;
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?;
        let expected = Bucket {
            user_id,
            tokens: 0,
            last_refill: now,
            action: LimitedAction::PublishNew,
        };
        assert_eq!(expected, bucket);

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?;
        assert_eq!(expected, bucket);
        Ok(())
    }

    #[test]
    fn a_user_with_no_tokens_gets_a_token_after_exactly_rate() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 0, now)?.user_id;
        let refill_time = now + chrono::Duration::seconds(1);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, conn)?;
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
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user_bucket(conn, 8, now)?.user_id;
        let refill_time = now + chrono::Duration::seconds(4);
        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, refill_time, conn)?;
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
    fn two_actions_dont_interfere_with_each_other() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let mut config = HashMap::new();
        config.insert(
            LimitedAction::PublishNew,
            RateLimiterConfig {
                rate: Duration::from_secs(1),
                burst: 10,
            },
        );
        config.insert(
            LimitedAction::YankUnyank,
            RateLimiterConfig {
                rate: Duration::from_secs(1),
                burst: 20,
            },
        );
        let rate = RateLimiter::new(config);

        let user_id = new_user(conn, "user")?;

        assert_eq!(
            10,
            rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?
                .tokens
        );
        assert_eq!(
            9,
            rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?
                .tokens
        );
        assert_eq!(
            20,
            rate.take_token(user_id, LimitedAction::YankUnyank, now, conn)?
                .tokens
        );

        Ok(())
    }

    #[test]
    fn override_is_used_instead_of_global_burst_if_present() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user(conn, "user1")?;
        let other_user_id = new_user(conn, "user2")?;

        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(user_id),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
                publish_rate_overrides::burst.eq(20),
            ))
            .execute(conn)?;

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?;
        let other_bucket = rate.take_token(other_user_id, LimitedAction::PublishNew, now, conn)?;

        assert_eq!(bucket.tokens, 20);
        assert_eq!(other_bucket.tokens, 10);
        Ok(())
    }

    #[test]
    fn overrides_can_expire() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();

        let rate = SampleRateLimiter {
            rate: Duration::from_secs(1),
            burst: 10,
            action: LimitedAction::PublishNew,
        }
        .create();
        let user_id = new_user(conn, "user1")?;
        let other_user_id = new_user(conn, "user2")?;

        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(user_id),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
                publish_rate_overrides::burst.eq(20),
                publish_rate_overrides::expires_at.eq(now + chrono::Duration::days(30)),
            ))
            .execute(conn)?;

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?;
        let other_bucket = rate.take_token(other_user_id, LimitedAction::PublishNew, now, conn)?;

        assert_eq!(bucket.tokens, 20);
        assert_eq!(other_bucket.tokens, 10);

        // Manually expire the rate limit
        diesel::update(publish_rate_overrides::table)
            .set(publish_rate_overrides::expires_at.eq(now - chrono::Duration::days(30)))
            .filter(publish_rate_overrides::user_id.eq(user_id))
            .execute(conn)?;

        let bucket = rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?;
        let other_bucket = rate.take_token(other_user_id, LimitedAction::PublishNew, now, conn)?;

        // The number of tokens of user_id is 10 and not 9 because when the new burst limit is
        // lower than the amount of available tokens, the number of available tokens is reset to
        // the new burst limit.
        assert_eq!(bucket.tokens, 10);
        assert_eq!(other_bucket.tokens, 9);

        Ok(())
    }

    #[test]
    fn override_is_different_for_each_action() -> QueryResult<()> {
        let (_test_db, conn) = &mut test_db_connection();
        let now = now();
        let user_id = new_user(conn, "user")?;

        let mut config = HashMap::new();
        for action in [LimitedAction::PublishNew, LimitedAction::YankUnyank] {
            config.insert(
                action,
                RateLimiterConfig {
                    rate: Duration::from_secs(1),
                    burst: 10,
                },
            );
        }
        let rate = RateLimiter::new(config);

        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(user_id),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
                publish_rate_overrides::burst.eq(20),
            ))
            .execute(conn)?;

        assert_eq!(
            20,
            rate.take_token(user_id, LimitedAction::PublishNew, now, conn)?
                .tokens,
        );
        assert_eq!(
            10,
            rate.take_token(user_id, LimitedAction::YankUnyank, now, conn)?
                .tokens,
        );

        Ok(())
    }

    fn new_user(conn: &mut impl Conn, gh_login: &str) -> QueryResult<i32> {
        use crate::models::NewUser;

        let user = NewUser {
            gh_login,
            ..NewUser::default()
        }
        .create_or_update(None, &Emails::new_in_memory(), conn)?;
        Ok(user.id)
    }

    fn new_user_bucket(
        conn: &mut impl Conn,
        tokens: i32,
        now: NaiveDateTime,
    ) -> QueryResult<Bucket> {
        diesel::insert_into(publish_limit_buckets::table)
            .values(Bucket {
                user_id: new_user(conn, "new_user")?,
                tokens,
                last_refill: now,
                action: LimitedAction::PublishNew,
            })
            .get_result(conn)
    }

    struct SampleRateLimiter {
        rate: Duration,
        burst: i32,
        action: LimitedAction,
    }

    impl SampleRateLimiter {
        fn create(self) -> RateLimiter {
            let mut config = HashMap::new();
            config.insert(
                self.action,
                RateLimiterConfig {
                    rate: self.rate,
                    burst: self.burst,
                },
            );
            RateLimiter::new(config)
        }
    }

    /// Strips ns precision from `Utc::now`. PostgreSQL only has microsecond
    /// precision, but some platforms (notably Linux) provide nanosecond
    /// precision, meaning that round tripping through the database would
    /// change the value.
    fn now() -> NaiveDateTime {
        let now = Utc::now();
        let nanos = now.timestamp_subsec_nanos();
        now.naive_utc() - chrono::Duration::nanoseconds(nanos.into())
    }
}

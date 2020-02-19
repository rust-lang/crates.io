use crate::background_jobs::Environment;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use swirl::PerformError;

table! {
    information_schema.tables (table_schema, table_name) {
        table_schema -> Text,
        table_name -> Text,
        table_type -> Text,
    }
}

#[derive(PartialEq, Debug)]
struct Quarter {
    start: NaiveDate,
    end: NaiveDate,
    num: u32,
}

impl Quarter {
    fn from_date(date: NaiveDate) -> Option<Self> {
        let num = (date.month() + 2) / 3;
        let start = date.with_day(1)?.with_month(num * 3 - 2)?;
        let end = (start + Duration::days(93)).with_day(1)?;
        Some(Self { num, start, end })
    }
}

#[swirl::background_job]
pub fn generate_version_downloads_partition(env: &Environment) -> Result<(), PerformError> {
    let conn = env.connection()?;
    generate_partition(&conn, Utc::today().naive_utc() + Duration::days(365))?;
    Ok(())
}

fn generate_partition(conn: &PgConnection, today: NaiveDate) -> QueryResult<()> {
    let quarter = Quarter::from_date(today).expect("could not determine start/end of quarter");
    let table_name = format!("version_downloads_{}_q{}", today.year(), quarter.num);

    if table_exists(conn, &table_name)? {
        Ok(())
    } else {
        sql_query(format!(
            "CREATE TABLE {} PARTITION OF version_downloads \
             FOR VALUES FROM ('{}') TO ('{}')",
            table_name, quarter.start, quarter.end
        ))
        .execute(conn)?;
        Ok(())
    }
}

fn table_exists(conn: &PgConnection, name: &str) -> QueryResult<bool> {
    use self::tables::dsl::*;
    use diesel::dsl::{exists, select};

    select(exists(
        tables
            .filter(table_schema.eq("public"))
            .filter(table_name.eq(name))
            .filter(table_type.like("BASE TABLE")),
    ))
    .get_result(conn)
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;

    #[test]
    fn generate_partition_creates_table_if_it_doesnt_exist() {
        let conn = conn();

        drop_table_if_exists(&conn, "version_downloads_2018_q1").unwrap();
        drop_table_if_exists(&conn, "version_downloads_2019_q2").unwrap();

        assert!(!table_exists(&conn, "version_downloads_2018_q1").unwrap());
        assert!(!table_exists(&conn, "version_downloads_2019_q2").unwrap());

        let q1_2018 = NaiveDate::from_ymd(2018, 2, 14);
        let q2_2019 = NaiveDate::from_ymd(2019, 4, 1);
        generate_partition(&conn, q1_2018).unwrap();

        assert!(table_exists(&conn, "version_downloads_2018_q1").unwrap());
        assert!(!table_exists(&conn, "version_downloads_2019_q2").unwrap());

        generate_partition(&conn, q2_2019).unwrap();

        assert!(table_exists(&conn, "version_downloads_2018_q1").unwrap());
        assert!(table_exists(&conn, "version_downloads_2019_q2").unwrap());
    }

    #[test]
    fn quarter_from_date() {
        let q1_2018 = NaiveDate::from_ymd(2018, 2, 14);
        let q4_2018 = NaiveDate::from_ymd(2018, 12, 1);
        let q1 = Quarter {
            start: NaiveDate::from_ymd(2018, 1, 1),
            end: NaiveDate::from_ymd(2018, 4, 1),
            num: 1,
        };
        let q4 = Quarter {
            start: NaiveDate::from_ymd(2018, 10, 1),
            end: NaiveDate::from_ymd(2019, 1, 1),
            num: 4,
        };

        assert_eq!(Some(q1), Quarter::from_date(q1_2018));
        assert_eq!(Some(q4), Quarter::from_date(q4_2018));
    }

    fn drop_table_if_exists(conn: &PgConnection, table_name: &str) -> QueryResult<()> {
        sql_query(&format!("DROP TABLE IF EXISTS {}", table_name)).execute(conn)?;
        Ok(())
    }
}

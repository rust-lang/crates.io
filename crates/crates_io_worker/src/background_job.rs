use crate::errors::EnqueueError;
use crate::schema::background_jobs;
use diesel::connection::LoadConnection;
use diesel::dsl::{exists, not};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Int2, Jsonb, Text};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use tracing::instrument;

pub const DEFAULT_QUEUE: &str = "default";

pub trait BackgroundJob: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Unique name of the task.
    ///
    /// This MUST be unique for the whole application.
    const JOB_NAME: &'static str;

    /// Default priority of the task.
    ///
    /// [Self::enqueue_with_priority] can be used to override the priority value.
    const PRIORITY: i16 = 0;

    /// Whether the job should be deduplicated.
    ///
    /// If true, the job will not be enqueued if there is already an unstarted
    /// job with the same data.
    const DEDUPLICATED: bool = false;

    /// Job queue where this job will be executed.
    const QUEUE: &'static str = DEFAULT_QUEUE;

    /// The application data provided to this job at runtime.
    type Context: Clone + Send + 'static;

    /// Execute the task. This method should define its logic.
    fn run(&self, ctx: Self::Context) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn enqueue(
        &self,
        conn: &mut impl LoadConnection<Backend = Pg>,
    ) -> Result<Option<i64>, EnqueueError> {
        self.enqueue_with_priority(conn, Self::PRIORITY)
    }

    #[instrument(name = "swirl.enqueue", skip(self, conn), fields(message = Self::JOB_NAME))]
    fn enqueue_with_priority(
        &self,
        conn: &mut impl LoadConnection<Backend = Pg>,
        priority: i16,
    ) -> Result<Option<i64>, EnqueueError> {
        let data = serde_json::to_value(self)?;

        if Self::DEDUPLICATED {
            Ok(enqueue_deduplicated(conn, Self::JOB_NAME, &data, priority)?)
        } else {
            Ok(Some(enqueue_simple(conn, Self::JOB_NAME, &data, priority)?))
        }
    }
}

fn enqueue_deduplicated(
    conn: &mut impl LoadConnection<Backend = Pg>,
    job_type: &str,
    data: &Value,
    priority: i16,
) -> Result<Option<i64>, EnqueueError> {
    let similar_jobs = background_jobs::table
        .select(background_jobs::id)
        .filter(background_jobs::job_type.eq(job_type))
        .filter(background_jobs::data.eq(data))
        .filter(background_jobs::priority.eq(priority))
        .for_update()
        .skip_locked();

    let deduplicated_select = diesel::select((
        job_type.into_sql::<Text>(),
        data.into_sql::<Jsonb>(),
        priority.into_sql::<Int2>(),
    ))
    .filter(not(exists(similar_jobs)));

    let id = diesel::insert_into(background_jobs::table)
        .values(deduplicated_select)
        .into_columns((
            background_jobs::job_type,
            background_jobs::data,
            background_jobs::priority,
        ))
        .returning(background_jobs::id)
        .get_result::<i64>(conn)
        .optional()?;

    Ok(id)
}

fn enqueue_simple(
    conn: &mut impl LoadConnection<Backend = Pg>,
    job_type: &str,
    data: &Value,
    priority: i16,
) -> Result<i64, EnqueueError> {
    let id = diesel::insert_into(background_jobs::table)
        .values((
            background_jobs::job_type.eq(job_type),
            background_jobs::data.eq(data),
            background_jobs::priority.eq(priority),
        ))
        .returning(background_jobs::id)
        .get_result(conn)?;

    Ok(id)
}

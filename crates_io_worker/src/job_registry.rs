use crate::BackgroundJob;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;

type RunTaskFn<Context> = dyn Fn(Context, serde_json::Value) -> anyhow::Result<()> + Send + Sync;

pub type JobRegistry<Context> = HashMap<String, Arc<RunTaskFn<Context>>>;

pub fn runnable<J: BackgroundJob>(
    ctx: J::Context,
    payload: serde_json::Value,
) -> anyhow::Result<()> {
    let job: J = serde_json::from_value(payload)?;
    Handle::current().block_on(job.run(ctx))
}

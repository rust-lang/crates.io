use crate::BackgroundJob;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type RunTaskFnReturn = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
type RunTaskFn<Context> = dyn Fn(Context, serde_json::Value) -> RunTaskFnReturn + Send + Sync;

#[derive(Clone)]
pub struct JobRegistry<Context> {
    entries: HashMap<String, Arc<RunTaskFn<Context>>>,
}

impl<Context> Default for JobRegistry<Context> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<Context: Clone + Send + 'static> JobRegistry<Context> {
    pub fn register<J: BackgroundJob<Context = Context>>(&mut self) {
        self.entries
            .insert(J::JOB_NAME.to_string(), Arc::new(runnable::<J>));
    }

    pub fn get(&self, key: &str) -> Option<&Arc<RunTaskFn<Context>>> {
        self.entries.get(key)
    }
}

fn runnable<J: BackgroundJob>(ctx: J::Context, payload: serde_json::Value) -> RunTaskFnReturn {
    Box::pin(async move {
        let job: J = serde_json::from_value(payload)?;
        job.run(ctx).await
    })
}

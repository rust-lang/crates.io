use crate::models::update_default_version;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct UpdateDefaultVersion {
    crate_id: i32,
}

impl UpdateDefaultVersion {
    pub fn new(crate_id: i32) -> Self {
        Self { crate_id }
    }
}

impl BackgroundJob for UpdateDefaultVersion {
    const JOB_NAME: &'static str = "update_default_version";
    const PRIORITY: i16 = 80;
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let crate_id = self.crate_id;

        info!("Updating default version for crate {crate_id}");
        let conn = ctx.deadpool.get().await?;
        spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
            update_default_version(crate_id, conn)?;
            Ok(())
        })
        .await
    }
}

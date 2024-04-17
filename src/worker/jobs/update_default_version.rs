use crate::models::update_default_version;
use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_worker::BackgroundJob;
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

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let crate_id = self.crate_id;

        info!("Updating default version for crate {crate_id}");
        let conn = ctx.deadpool.get().await?;
        conn.interact::<_, anyhow::Result<_>>(move |conn| {
            update_default_version(crate_id, conn)?;
            Ok(())
        })
        .await
        .map_err(|err| anyhow!(err.to_string()))??;

        Ok(())
    }
}

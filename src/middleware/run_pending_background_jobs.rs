use super::app::RequestApp;
use super::prelude::*;
use crate::background::Runner;
use crate::background_jobs::*;

pub struct RunPendingBackgroundJobs;

impl Middleware for RunPendingBackgroundJobs {
    fn after(
        &self,
        req: &mut dyn Request,
        res: Result<Response, Box<dyn Error + Send>>,
    ) -> Result<Response, Box<dyn Error + Send>> {
        let app = req.app();
        let connection_pool = app.diesel_database.clone();
        let environment = Environment {
            index_location: app.config.index_location.clone(),
            credentials: None,
        };

        let config = Runner::builder(connection_pool, environment);
        let runner = job_runner(config);

        runner.run_all_pending_jobs().expect("Could not run jobs");
        runner
            .assert_no_failed_jobs()
            .expect("Could not determine if jobs failed");
        res
    }
}

use swirl::Runner;

use super::app::RequestApp;
use super::prelude::*;
use crate::background_jobs::*;
use crate::git::Repository;

pub struct RunPendingBackgroundJobs;

impl Middleware for RunPendingBackgroundJobs {
    fn after(
        &self,
        req: &mut dyn Request,
        res: Result<Response, Box<dyn Error + Send>>,
    ) -> Result<Response, Box<dyn Error + Send>> {
        if response_is_error(&res) {
            return res;
        }

        let app = req.app();
        let connection_pool = app.diesel_database.clone();
        let repo = Repository::open(&app.config.index_location).expect("Could not clone index");
        let environment = Environment::new(repo, None, app.diesel_database.clone());

        let config = Runner::builder(connection_pool, environment);
        let runner = job_runner(config);

        runner.run_all_pending_jobs().expect("Could not run jobs");
        runner
            .assert_no_failed_jobs()
            .expect("Could not determine if jobs failed");
        res
    }
}

fn response_is_error(res: &Result<Response, Box<dyn Error + Send>>) -> bool {
    match res {
        Ok(res) => res.status.0 >= 400,
        Err(_) => true,
    }
}

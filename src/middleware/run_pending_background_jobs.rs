use std::time::Duration;
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

        let runner = Runner::builder(connection_pool, environment)
            // We only have 1 connection in tests, so trying to run more than
            // 1 job concurrently will just block
            .thread_count(1)
            .job_start_timeout(Duration::from_secs(1))
            .build();

        // FIXME: https://github.com/sgrif/swirl/issues/8
        if let Err(e) = runner.run_all_pending_jobs() {
            if e.to_string().ends_with("read-only transaction") {
                return res;
            } else {
                panic!("Could not run jobs: {}", e);
            }
        }

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

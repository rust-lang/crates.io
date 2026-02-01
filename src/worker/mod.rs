//! This module contains the code for the background jobs that run on the
//! crates.io backend servers.
//!
//! The `swirl` submodule contains the code for the generic background job
//! runner, and the `jobs` submodule contains the application-specific
//! background job definitions.

use crates_io_worker::Runner;
use std::sync::Arc;

mod environment;
pub mod jobs;

pub use self::environment::Environment;

pub trait RunnerExt {
    fn register_crates_io_job_types(self) -> Self;
}

impl RunnerExt for Runner<Arc<Environment>> {
    fn register_crates_io_job_types(self) -> Self {
        self.register_job_type::<jobs::AnalyzeCrateFile>()
            .register_job_type::<jobs::ArchiveVersionDownloads>()
            .register_job_type::<jobs::BulkSyncToGitIndex>()
            .register_job_type::<jobs::CheckTyposquat>()
            .register_job_type::<jobs::CleanProcessedLogFiles>()
            .register_job_type::<jobs::DailyDbMaintenance>()
            .register_job_type::<jobs::DeleteCrateFromStorage>()
            .register_job_type::<jobs::DocsRsQueueRebuild>()
            .register_job_type::<jobs::DumpDb>()
            .register_job_type::<jobs::GenerateOgImage>()
            .register_job_type::<jobs::IndexVersionDownloadsArchive>()
            .register_job_type::<jobs::InvalidateCdns>()
            .register_job_type::<jobs::NormalizeIndex>()
            .register_job_type::<jobs::ProcessCdnLog>()
            .register_job_type::<jobs::ProcessCdnLogQueue>()
            .register_job_type::<jobs::ProcessCloudfrontInvalidationQueue>()
            .register_job_type::<jobs::RenderAndUploadReadme>()
            .register_job_type::<jobs::SquashIndex>()
            .register_job_type::<jobs::SyncAdmins>()
            .register_job_type::<jobs::SyncToGitIndex>()
            .register_job_type::<jobs::SyncToSparseIndex>()
            .register_job_type::<jobs::UpdateDownloads>()
            .register_job_type::<jobs::UpdateDefaultVersion>()
            .register_job_type::<jobs::SendTokenExpiryNotifications>()
            .register_job_type::<jobs::SendPublishNotificationsJob>()
            .register_job_type::<jobs::rss::SyncCrateFeed>()
            .register_job_type::<jobs::rss::SyncCratesFeed>()
            .register_job_type::<jobs::rss::SyncUpdatesFeed>()
            .register_job_type::<jobs::trustpub::DeleteExpiredJtis>()
            .register_job_type::<jobs::trustpub::DeleteExpiredTokens>()
    }
}

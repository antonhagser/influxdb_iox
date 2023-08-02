//! Used as wrapper for [`LocalScheduler`](crate::LocalScheduler) and RemoteScheduler(TBD).

use async_trait::async_trait;

use crate::{
    CompactionJob, CompactionJobEnd, CompactionJobStatus, CompactionJobStatusResponse, Scheduler,
};

/// Wrapper that enforces job tracking.
#[derive(Debug)]
pub(crate) struct JobTracker<T>
where
    T: Scheduler + std::fmt::Display + Send + Sync,
{
    inner: T,
}

impl<T> JobTracker<T>
where
    T: Scheduler + std::fmt::Display + Send + Sync,
{
    /// Create a new job tracker.
    pub(crate) fn new(inner: T) -> Self {
        Self { inner }
    }
}

/// Wrap the scheduler interface.
#[async_trait]
impl<T> Scheduler for JobTracker<T>
where
    T: Scheduler + std::fmt::Display + Send + Sync,
{
    /// Track the jobs that are created.
    async fn get_jobs(&self) -> Vec<CompactionJob> {
        self.inner.get_jobs().await
    }

    /// Pass through to the inner scheduler.
    async fn update_job_status(
        &self,
        job_status: CompactionJobStatus,
    ) -> Result<CompactionJobStatusResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.update_job_status(job_status).await
    }

    /// Track the jobs that are ended.
    async fn end_job(
        &self,
        end_action: CompactionJobEnd,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.end_job(end_action).await
    }
}

impl<T> std::fmt::Display for JobTracker<T>
where
    T: Scheduler + std::fmt::Display + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "job_tracker({})", self.inner)
    }
}

//! Used as wrapper for [`LocalScheduler`](crate::LocalScheduler) and RemoteScheduler(TBD).

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use data_types::PartitionId;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{
    CompactionJob, CompactionJobEnd, CompactionJobStatus, CompactionJobStatusResponse, Identity,
    Scheduler,
};

/// Error returned by job tracker.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Unknown job.
    #[error("Unknown or already done compaction_job={0} for partition_id={1}")]
    Unknown(Uuid, PartitionId),
}

/// Wrapper that enforces job tracking.
#[derive(Debug)]
pub(crate) struct JobTracker<T>
where
    T: Scheduler + std::fmt::Display + Send + Sync,
{
    inner: T,
    in_flight_jobs: Mutex<HashMap<Uuid, PartitionId>>,
    in_flight_partitions: Mutex<HashSet<PartitionId>>,
}

impl<T> JobTracker<T>
where
    T: Scheduler + std::fmt::Display + Send + Sync,
{
    /// Create a new job tracker.
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            in_flight_jobs: Mutex::new(HashMap::new()),
            in_flight_partitions: Mutex::new(HashSet::new()),
        }
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
        self.inner
            .get_jobs()
            .await
            .into_iter()
            .map(|job| {
                let partition_exists = !self.in_flight_partitions.lock().insert(job.partition_id);
                let job_exists = self
                    .in_flight_jobs
                    .lock()
                    .insert(job.uuid(), job.partition_id)
                    .is_some();
                if self.identity() == Identity::Local && (partition_exists || job_exists) {
                    unreachable!(
                        "JobTracker::get_jobs() called with duplicate job: {:?}",
                        job
                    );
                }
                // TODO: if RemoteScheduler gave a duplicate, then give up lease for this job.
                // TODO: need RemoteScheduler interface ability to state how much of the job criteria has complete. (a.k.a. not done)

                job
            })
            .collect::<Vec<CompactionJob>>()
    }

    /// Pass through to the inner scheduler.
    async fn update_job_status(
        &self,
        job_status: CompactionJobStatus,
    ) -> Result<CompactionJobStatusResponse, Box<dyn std::error::Error + Send + Sync>> {
        let has_lease = self
            .in_flight_jobs
            .lock()
            .contains_key(&job_status.job.uuid())
            && self
                .in_flight_partitions
                .lock()
                .contains(&job_status.job.partition_id);

        if has_lease {
            self.inner.update_job_status(job_status).await
        } else {
            Err(Error::Unknown(job_status.job.uuid(), job_status.job.partition_id).into())
        }
    }

    /// Track the jobs that are ended.
    async fn end_job(
        &self,
        end_action: CompactionJobEnd,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let job = end_action.job.clone();

        let has_lease = self.in_flight_partitions.lock().remove(&job.partition_id)
            && self.in_flight_jobs.lock().remove(&job.uuid()).is_some();
        if !has_lease {
            return Err(Error::Unknown(end_action.job.uuid(), end_action.job.partition_id).into());
        }

        self.inner.end_job(end_action).await
    }

    fn identity(&self) -> crate::Identity {
        self.inner.identity()
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

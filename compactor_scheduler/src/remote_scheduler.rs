use async_trait::async_trait;

use crate::scheduler::{CompactionJob, Scheduler};

/// Implementation of the [`Scheduler`] for remote coordinated scheduling.
#[derive(Debug)]
pub(crate) struct RemoteScheduler;

impl RemoteScheduler {
    /// Create new RemoteScheduler.
    pub(crate) fn new() -> Self {
        unimplemented!("RemoteScheduler is not yet implemented")
    }

    #[allow(dead_code)]
    /// Attempt to connect to remote scheduler.
    pub(crate) async fn connect(&self) -> Result<Self, Box<dyn std::error::Error>> {
        unimplemented!("remote compactor coordinator & scheduler service is not yet implemented");
    }

    #[allow(dead_code)]
    async fn heart_beat(&self) -> Result<Self, Box<dyn std::error::Error>> {
        unimplemented!("determine liveness communication protocol");
    }

    #[allow(dead_code)]
    async fn graceful_shutdown(&self) -> Result<Self, Box<dyn std::error::Error>> {
        unimplemented!("determine liveness communication protocol");
    }
}

#[async_trait]
impl Scheduler for RemoteScheduler {
    async fn get_job(&self) -> Vec<CompactionJob> {
        unimplemented!("connect to grpc server and get jobs");
    }
}

impl std::fmt::Display for RemoteScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "remote_compaction_scheduler")
    }
}

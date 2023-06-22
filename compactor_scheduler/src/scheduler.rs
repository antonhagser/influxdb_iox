use std::fmt::{Debug, Display};

use async_trait::async_trait;
use data_types::PartitionId;

/// Job assignment for a given partition.
#[derive(Debug)]
pub struct CompactionJob {
    /// Leased partition.
    pub partition_id: PartitionId,
}

/// Core trait used for all schedulers.
#[async_trait]
pub trait Scheduler: Send + Sync + Debug + Display {
    /// Get partitions to be compacted.
    async fn get_job(&self) -> Vec<CompactionJob>;
}

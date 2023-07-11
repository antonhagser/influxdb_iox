pub(crate) mod catalog;
pub(crate) mod mock;

use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use async_trait::async_trait;
use data_types::PartitionId;

use crate::CompactionJob;

/// Dynamic error type that is used throughout the stack.
pub(crate) type DynError = Box<dyn std::error::Error + Send + Sync>;

/// Records "partition is done" status for given partition.
#[async_trait]
pub trait PartitionDoneSink<P>: Debug + Display + Send + Sync
where
    P: Into<PartitionId> + Send + Sync + std::fmt::Debug,
{
    /// Record "partition is done" status for given partition.
    ///
    /// This method should retry.
    async fn record(&self, partition: P, res: Result<(), DynError>);
}

#[async_trait]
impl<T> PartitionDoneSink<PartitionId> for Arc<T>
where
    T: PartitionDoneSink<PartitionId> + ?Sized,
{
    async fn record(&self, partition: PartitionId, res: Result<(), DynError>) {
        self.as_ref().record(partition, res).await
    }
}

#[async_trait]
impl<T> PartitionDoneSink<CompactionJob> for Arc<T>
where
    T: PartitionDoneSink<CompactionJob> + ?Sized,
{
    async fn record(&self, partition: CompactionJob, res: Result<(), DynError>) {
        self.as_ref().record(partition, res).await
    }
}

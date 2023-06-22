//! service for scheduling compactor tasks.

#![deny(
    rustdoc::broken_intra_doc_links,
    rust_2018_idioms,
    missing_debug_implementations,
    unreachable_pub
)]
#![warn(
    missing_docs,
    clippy::todo,
    clippy::dbg_macro,
    clippy::explicit_iter_loop,
    clippy::clone_on_ref_ptr,
    // See https://github.com/influxdata/influxdb_iox/pull/1671
    clippy::future_not_send,
    unused_crate_dependencies
)]
#![allow(clippy::missing_docs_in_private_items)]

use backoff::BackoffConfig;
use iox_time::TimeProvider;
// Workaround for "unused crate" lint false positives.
use workspace_hack as _;

mod local_scheduler;
pub(crate) use local_scheduler::LocalScheduler;
mod partitions_source;
pub use partitions_source::*;
mod scheduler;
pub use scheduler::*;

use std::sync::Arc;

use clap_blocks::compactor_scheduler::{CompactorSchedulerConfig, CompactorSchedulerType};
use iox_catalog::interface::Catalog;

use crate::local_scheduler::{
    partitions_source_config::PartitionsSourceConfig, shard_config::ShardConfig,
};

/// Instantiate a compaction scheduler service
pub fn create_compactor_scheduler_service(
    scheduler_config: CompactorSchedulerConfig,
    catalog: Arc<dyn Catalog>,
    time_provider: Arc<dyn TimeProvider>,
) -> Arc<dyn Scheduler> {
    match scheduler_config.compactor_scheduler_type {
        CompactorSchedulerType::Local => {
            let scheduler = LocalScheduler::new(
                PartitionsSourceConfig::from_config(scheduler_config.partition_source_config),
                ShardConfig::from_config(scheduler_config.shard_config),
                BackoffConfig::default(),
                catalog,
                time_provider,
            );
            Arc::new(scheduler)
        }
        CompactorSchedulerType::Remote => unimplemented!("Remote scheduler not implemented"),
    }
}

/// Create a new [`LocalScheduler`] for testing.
pub fn create_test_compactor_scheduler(
    catalog: Arc<dyn Catalog>,
    time_provider: Arc<dyn TimeProvider>,
) -> Arc<dyn Scheduler> {
    Arc::new(LocalScheduler::new(
        PartitionsSourceConfig::default(),
        None,
        BackoffConfig::default(),
        catalog,
        time_provider,
    ))
}

// Temporary exports. Will eventually be encapsulated in local_scheduler.
pub use local_scheduler::id_only_partition_filter::IdOnlyPartitionFilter;

use std::fmt::{Debug, Display};

use async_trait::async_trait;

use crate::scheduler::{CompactionJob, Scheduler};

/// Implementation of the [`Scheduler`] for local (per compactor) scheduling.
#[derive(Debug, Default)]
pub struct LocalScheduler;

#[async_trait]
impl Scheduler for LocalScheduler {
    async fn get_job(&self) -> Vec<CompactionJob> {
        return Vec::new();
    }
}

impl Display for LocalScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "local_compaction_scheduler",)
    }
}

//! Used in the creation of a [`PartitionsSource`](crate::PartitionsSource) of PartitionIds
//! for the [`LocalScheduler`](crate::LocalScheduler).
pub(crate) mod catalog_all;
pub(crate) mod catalog_to_compact;
pub(crate) mod filter;

use std::sync::Arc;

use datafusion::{execution::context::SessionState, physical_optimizer::{PhysicalOptimizerRule, sort_enforcement::EnforceSorting}};

use self::{
    combine_chunks::CombineChunks,
    dedup::{
        dedup_null_columns::DedupNullColumns, dedup_sort_order::DedupSortOrder,
        partition_split::PartitionSplit, remove_dedup::RemoveDedup, time_split::TimeSplit,
    },
    predicate_pushdown::PredicatePushdown,
    projection_pushdown::ProjectionPushdown,
    sort::{parquet_sortness::ParquetSortness, push_sort_through_union::PushSortThroughUnion},
    union::{nested_union::NestedUnion, one_union::OneUnion},
};

mod chunk_extraction;
mod combine_chunks;
mod dedup;
mod predicate_pushdown;
mod projection_pushdown;
mod sort;
mod union;

#[cfg(test)]
mod test_util;

/// Register IOx-specific [`PhysicalOptimizerRule`]s with the SessionContext
pub fn register_iox_physical_optimizers(state: SessionState) -> SessionState {
    // prepend IOx-specific rules to DataFusion builtins
    // The optimizer rules have to be done in this order
    let mut optimizers: Vec<Arc<dyn PhysicalOptimizerRule + Sync + Send>> = vec![
        Arc::new(PartitionSplit),
        Arc::new(TimeSplit),
        Arc::new(RemoveDedup),
        Arc::new(CombineChunks),
        Arc::new(DedupNullColumns),
        Arc::new(DedupSortOrder),
        Arc::new(PredicatePushdown),
        Arc::new(ProjectionPushdown),
        Arc::new(ParquetSortness) as _,
        Arc::new(NestedUnion),
        Arc::new(OneUnion),
    ];
    optimizers.append(&mut state.physical_optimizers().to_vec());

    // Add a PushSortThroughUnion just after EnforceSorting
    let enforce_sorting = EnforceSorting::new();
    let enforce_sorting_name = enforce_sorting.name();
    let (enforce_sorting_index, _) = optimizers.iter().enumerate()
        .find(|(_, rule)| rule.name() == enforce_sorting_name)
        .expect("EnforceSorting should be present");
    optimizers.insert(enforce_sorting_index + 1, Arc::new(PushSortThroughUnion));

    state.with_physical_optimizer_rules(optimizers)
}

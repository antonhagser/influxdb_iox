//! Database for the querier that contains all namespaces.

use crate::{
    cache::CatalogCache,
    ingester::IngesterConnection,
    namespace::{QuerierNamespace, QuerierNamespaceArgs},
    parquet::ChunkAdapter,
    query_log::QueryLog,
    table::PruneMetrics,
};
use async_trait::async_trait;
use backoff::{Backoff, BackoffConfig};
use data_types::Namespace;
use iox_catalog::interface::SoftDeletedRows;
use iox_query::exec::Executor;
use service_common::QueryNamespaceProvider;
use snafu::Snafu;
use std::{collections::HashMap, sync::Arc};
use trace::span::{Span, SpanRecorder};
use tracker::{
    AsyncSemaphoreMetrics, InstrumentedAsyncOwnedSemaphorePermit, InstrumentedAsyncSemaphore,
};

/// The number of entries to store in the circular query buffer log.
///
/// That buffer is shared between all namespaces, and filtered on query
const QUERY_LOG_SIZE: usize = 10_000;

#[allow(missing_docs)]
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Catalog error: {source}"))]
    Catalog {
        source: iox_catalog::interface::Error,
    },
}

/// Database for the querier.
///
/// Contains all namespaces.
#[derive(Debug)]
pub struct QuerierDatabase {
    /// Backoff config for IO operations.
    backoff_config: BackoffConfig,

    /// Catalog cache.
    catalog_cache: Arc<CatalogCache>,

    /// Adapter to create chunks.
    chunk_adapter: Arc<ChunkAdapter>,

    /// Metric registry
    #[allow(dead_code)]
    metric_registry: Arc<metric::Registry>,

    /// Executor for queries.
    exec: Arc<Executor>,

    /// Connection to ingester(s)
    ingester_connection: Option<Arc<dyn IngesterConnection>>,

    /// Query log.
    query_log: Arc<QueryLog>,

    /// Semaphore that limits the number of namespaces in used at the time by the query subsystem.
    ///
    /// This should be a 1-to-1 relation to the number of active queries.
    ///
    /// If the same namespace is requested twice for different queries, it is counted twice.
    query_execution_semaphore: Arc<InstrumentedAsyncSemaphore>,

    /// Chunk prune metrics.
    prune_metrics: Arc<PruneMetrics>,

    /// DataFusion config.
    datafusion_config: Arc<HashMap<String, String>>,
}

#[async_trait]
impl QueryNamespaceProvider for QuerierDatabase {
    type Db = QuerierNamespace;

    async fn db(
        &self,
        name: &str,
        span: Option<Span>,
        include_debug_info_tables: bool,
    ) -> Option<Arc<Self::Db>> {
        self.namespace(name, span, include_debug_info_tables).await
    }

    async fn acquire_semaphore(&self, span: Option<Span>) -> InstrumentedAsyncOwnedSemaphorePermit {
        Arc::clone(&self.query_execution_semaphore)
            .acquire_owned(span)
            .await
            .expect("Semaphore should not be closed by anyone")
    }
}

impl QuerierDatabase {
    /// The maximum value for `max_concurrent_queries` that is allowed.
    ///
    /// This limit exists because [`tokio::sync::Semaphore`] has an internal limit and semaphore
    /// creation beyond that will panic. The tokio limit is not exposed though so we pick a
    /// reasonable but smaller number.
    pub const MAX_CONCURRENT_QUERIES_MAX: usize = u16::MAX as usize;

    /// Create new database.
    pub async fn new(
        catalog_cache: Arc<CatalogCache>,
        metric_registry: Arc<metric::Registry>,
        exec: Arc<Executor>,
        ingester_connection: Option<Arc<dyn IngesterConnection>>,
        max_concurrent_queries: usize,
        datafusion_config: Arc<HashMap<String, String>>,
    ) -> Result<Self, Error> {
        assert!(
            max_concurrent_queries <= Self::MAX_CONCURRENT_QUERIES_MAX,
            "`max_concurrent_queries` ({}) > `max_concurrent_queries_MAX` ({})",
            max_concurrent_queries,
            Self::MAX_CONCURRENT_QUERIES_MAX,
        );

        let backoff_config = BackoffConfig::default();

        let chunk_adapter = Arc::new(ChunkAdapter::new(
            Arc::clone(&catalog_cache),
            Arc::clone(&metric_registry),
        ));
        let query_log = Arc::new(QueryLog::new(QUERY_LOG_SIZE, catalog_cache.time_provider()));
        let semaphore_metrics = Arc::new(AsyncSemaphoreMetrics::new(
            &metric_registry,
            &[("semaphore", "query_execution")],
        ));
        let query_execution_semaphore =
            Arc::new(semaphore_metrics.new_semaphore(max_concurrent_queries));

        let prune_metrics = Arc::new(PruneMetrics::new(&metric_registry));

        Ok(Self {
            backoff_config,
            catalog_cache,
            chunk_adapter,
            metric_registry,
            exec,
            ingester_connection,
            query_log,
            query_execution_semaphore,
            prune_metrics,
            datafusion_config,
        })
    }

    /// Get namespace if it exists.
    ///
    /// This will await the internal namespace semaphore. Existence of namespaces is checked AFTER
    /// a semaphore permit was acquired since this lowers the chance that we obtain stale data.
    pub async fn namespace(
        &self,
        name: &str,
        span: Option<Span>,
        include_debug_info_tables: bool,
    ) -> Option<Arc<QuerierNamespace>> {
        let span_recorder = SpanRecorder::new(span);
        let name = Arc::from(name.to_owned());
        let ns = self
            .catalog_cache
            .namespace()
            .get(
                Arc::clone(&name),
                // we have no specific need for any tables or columns at this point, so nothing to cover
                &[],
                span_recorder.child_span("cache GET namespace schema"),
            )
            .await?;
        Some(Arc::new(QuerierNamespace::new(QuerierNamespaceArgs {
            chunk_adapter: Arc::clone(&self.chunk_adapter),
            ns,
            name,
            exec: Arc::clone(&self.exec),
            ingester_connection: self.ingester_connection.clone(),
            query_log: Arc::clone(&self.query_log),
            prune_metrics: Arc::clone(&self.prune_metrics),
            datafusion_config: Arc::clone(&self.datafusion_config),
            include_debug_info_tables,
        })))
    }

    /// Return all namespaces this querier knows about
    pub async fn namespaces(&self) -> Vec<Namespace> {
        let catalog = &self.catalog_cache.catalog();
        Backoff::new(&self.backoff_config)
            .retry_all_errors("listing namespaces", || async {
                catalog
                    .repositories()
                    .await
                    .namespaces()
                    .list(SoftDeletedRows::ExcludeDeleted)
                    .await
            })
            .await
            .expect("retry forever")
    }

    /// Return connection to ingester(s) to get and aggregate information from them
    pub fn ingester_connection(&self) -> Option<Arc<dyn IngesterConnection>> {
        self.ingester_connection.clone()
    }

    /// Executor
    pub(crate) fn exec(&self) -> &Executor {
        &self.exec
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, time::Instant};

    use super::*;
    use crate::{create_ingester_connection_for_testing, table::MetricPruningObserver};
    use arrow::util::pretty::pretty_format_batches;
    use datafusion::{
        common::tree_node::TreeNode,
        config::ConfigOptions,
        physical_expr::PhysicalSortExpr,
        physical_optimizer::{
            dist_enforcement::EnforceDistribution, sort_enforcement::EnforceSorting,
            PhysicalOptimizerRule,
        },
        physical_plan::{
            file_format::ParquetExec, planner::DefaultPhysicalPlanner, ExecutionPlan,
            PhysicalPlanner,
        },
        prelude::{count, lit, Expr},
    };
    use iox_catalog::{
        interface::Catalog,
        sqlite::{SqliteCatalog, SqliteConnectionOptions},
    };
    use iox_query::{
        exec::{ExecutionContextProvider, ExecutorConfig, IOxSessionContext},
        provider::DeduplicateExec,
        test::{format_execution_plan, format_logical_plan},
        QueryChunk, ScanPlanBuilder,
    };
    use iox_tests::TestCatalog;
    use iox_time::SystemProvider;
    use predicate::Predicate;
    use schema::Schema;
    use tokio::runtime::Handle;

    #[tokio::test]
    #[should_panic(
        expected = "`max_concurrent_queries` (65536) > `max_concurrent_queries_MAX` (65535)"
    )]
    async fn test_semaphore_limit_is_checked() {
        let catalog = TestCatalog::new();

        let catalog_cache = Arc::new(CatalogCache::new_testing(
            catalog.catalog(),
            catalog.time_provider(),
            catalog.metric_registry(),
            catalog.object_store(),
            &Handle::current(),
        ));
        QuerierDatabase::new(
            catalog_cache,
            catalog.metric_registry(),
            catalog.exec(),
            Some(create_ingester_connection_for_testing()),
            QuerierDatabase::MAX_CONCURRENT_QUERIES_MAX.saturating_add(1),
            Arc::new(HashMap::default()),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_namespace() {
        let catalog = TestCatalog::new();
        let db = new_db(&catalog).await;

        catalog.create_namespace_1hr_retention("ns1").await;

        assert!(db.namespace("ns1", None, true).await.is_some());
        assert!(db.namespace("ns2", None, true).await.is_none());
    }

    #[tokio::test]
    async fn test_namespaces() {
        let catalog = TestCatalog::new();
        let db = new_db(&catalog).await;

        catalog.create_namespace_1hr_retention("ns1").await;
        catalog.create_namespace_1hr_retention("ns2").await;

        let mut namespaces = db.namespaces().await;
        namespaces.sort_by_key(|ns| ns.name.clone());
        assert_eq!(namespaces.len(), 2);
        assert_eq!(namespaces[0].name, "ns1");
        assert_eq!(namespaces[1].name, "ns2");
    }

    async fn new_db(catalog: &Arc<TestCatalog>) -> QuerierDatabase {
        let catalog_cache = Arc::new(CatalogCache::new_testing(
            catalog.catalog(),
            catalog.time_provider(),
            catalog.metric_registry(),
            catalog.object_store(),
            &Handle::current(),
        ));
        QuerierDatabase::new(
            catalog_cache,
            catalog.metric_registry(),
            catalog.exec(),
            Some(create_ingester_connection_for_testing()),
            QuerierDatabase::MAX_CONCURRENT_QUERIES_MAX,
            Arc::new(HashMap::default()),
        )
        .await
        .unwrap()
    }

    async fn new_sqllite_catalog(registry: Arc<metric::Registry>, path: &str) -> Arc<CatalogCache> {
        let path: &str = path.as_ref();

        let sqlite_options = SqliteConnectionOptions {
            file_path: format!("{path}catalog.sqlite"),
        };
        let catalog: Arc<dyn Catalog> = Arc::new(
            SqliteCatalog::connect(sqlite_options, Arc::clone(&registry))
                .await
                .unwrap(),
        );
        let time_provider = Arc::new(SystemProvider::new());
        let object_store = Arc::new(
            object_store::local::LocalFileSystem::new_with_prefix(format!("{path}object_store"))
                .unwrap(),
        );
        Arc::new(CatalogCache::new_testing(
            Arc::clone(&catalog),
            time_provider,
            Arc::clone(&registry),
            Arc::clone(&object_store) as _,
            &Handle::current(),
        ))
    }

    async fn new_local_querier_database(
        registry: Arc<metric::Registry>,
        catalog_cache: Arc<CatalogCache>,
    ) -> Result<QuerierDatabase, Error> {
        let num_threads = NonZeroUsize::new(12).unwrap();
        let parquet_store = &catalog_cache.parquet_store();
        let executor = Arc::new(Executor::new_with_config(ExecutorConfig {
            num_threads,
            target_query_partitions: num_threads,
            object_stores: HashMap::from([(
                parquet_store.id(),
                Arc::clone(parquet_store.object_store()),
            )]),
            mem_pool_size: 32_589_934_592,
        }));
        QuerierDatabase::new(
            catalog_cache,
            registry,
            executor,
            None,
            QuerierDatabase::MAX_CONCURRENT_QUERIES_MAX,
            Arc::new(HashMap::default()),
        )
        .await
    }

    #[tokio::test]
    async fn test_parallel_dedup() {
        test_helpers::maybe_start_logging();
        let registry = Arc::new(metric::Registry::new());
        let catalog_cache =
            new_sqllite_catalog(Arc::clone(&registry), "/Users/cwolff/w/dedup-perf/data/").await;
        let qdb = new_local_querier_database(Arc::clone(&registry), Arc::clone(&catalog_cache))
            .await
            .unwrap();

        let db_name = "4ffa1256cf09608b_2b62db476a21c690";
        let table_name = "incoming_lines.count";

        let db = &qdb.db(db_name, None, false).await.unwrap();
        let ctx = db.new_query_context(None);

        let count_stmt = r#"select count(*) from "incoming_lines.count""#;
        println!("*** planning and executing {count_stmt} ***");
        let phys_plan = &ctx.sql_to_physical_plan(count_stmt).await.unwrap();
        for line in format_execution_plan(phys_plan) {
            println!("{line}");
        }
        exec_plan(&ctx, Arc::clone(phys_plan)).await;

        // Get the largest parquet file for the table
        let ns_cache = catalog_cache.namespace();
        let ns = ns_cache.get(db_name.into(), &[], None).await.unwrap();
        let table = &ns.tables[table_name.into()];
        let pq_cache = &catalog_cache.parquet_file();
        let pq_files = pq_cache.get(table.id, None, None).await;
        let pq_file = pq_files
            .files
            .iter()
            .max_by_key(|f| f.file_size_bytes)
            .unwrap();
        println!("largest parquet file has UUID {}", pq_file.object_store_id);

        // Create a chunk
        let chunk_adapter = ChunkAdapter::new(Arc::clone(&catalog_cache), Arc::clone(&registry));
        let prune_metrics = Arc::new(PruneMetrics::new(&chunk_adapter.metric_registry()));
        let mp_observer = MetricPruningObserver::new(prune_metrics);
        let predicate = Predicate::default();
        let chunks: Vec<Arc<dyn QueryChunk>> = chunk_adapter
            .new_chunks(
                Arc::clone(table),
                Arc::new(vec![Arc::clone(pq_file)]),
                &predicate,
                mp_observer,
                None,
            )
            .await
            .into_iter()
            .map(|pq_chunk| Arc::new(pq_chunk) as _)
            .collect();

        build_and_execute(&ctx, false, table_name, &table.schema, chunks.clone()).await;
        build_and_execute(&ctx, true, table_name, &table.schema, chunks.clone()).await;
    }

    async fn build_and_execute(
        ctx: &IOxSessionContext,
        allow_parallel: bool,
        table_name: &'static str,
        schema: &Schema,
        chunks: Vec<Arc<dyn QueryChunk>>,
    ) {
        println!("*** allow parallel: {allow_parallel} ***");
        let logical_plan_builder = ScanPlanBuilder::new(table_name.into(), &schema)
            .with_chunks(chunks)
            .build()
            .unwrap()
            .plan_builder;
        let logical_plan = logical_plan_builder
            .aggregate(Vec::<Expr>::new(), [count(lit(1))])
            .unwrap()
            .build()
            .unwrap();
        println!("    *** logical plan");
        for line in format_logical_plan(&logical_plan) {
            println!("{line}");
        }

        // don't use the context to create the physical plan here, since it will include
        // physical optimizers that will optimize away deduplication.
        let plan = DefaultPhysicalPlanner::default()
            .create_physical_plan(&logical_plan, &ctx.inner().state())
            .await
            .unwrap();
        println!("    *** initial physical plan");
        for line in format_execution_plan(&plan) {
            println!("{line}");
        }
        let plan = inject_dedup(plan, allow_parallel);
        exec_plan(&ctx, Arc::clone(&plan)).await;
    }

    async fn exec_plan(ctx: &IOxSessionContext, plan: Arc<dyn ExecutionPlan>) {
        let start = Instant::now();
        let batches = ctx.collect(Arc::clone(&plan)).await.unwrap();
        let elapsed = start.elapsed();
        println!("Duration: {}s", elapsed.as_secs_f64());
        println!("{}", pretty_format_batches(&batches).unwrap());
    }

    fn inject_dedup(node: Arc<dyn ExecutionPlan>, allow_parallel: bool) -> Arc<dyn ExecutionPlan> {
        fn inject_dedup_impl(
            node: Arc<dyn ExecutionPlan>,
            allow_parallel: bool,
        ) -> Arc<dyn ExecutionPlan> {
            if let Some(pq_exec) = node.as_any().downcast_ref::<ParquetExec>() {
                let sort_exprs: Vec<PhysicalSortExpr> =
                    pq_exec.output_ordering().unwrap().iter().cloned().collect();
                let dedup_exec = if allow_parallel {
                    DeduplicateExec::new_allow_parallel(
                        Arc::new(pq_exec.clone()),
                        sort_exprs,
                        false,
                    )
                } else {
                    DeduplicateExec::new(Arc::new(pq_exec.clone()), sort_exprs, false)
                };
                Arc::new(dedup_exec)
            } else {
                node.map_children(|n| Ok(inject_dedup_impl(n, allow_parallel)))
                    .unwrap()
            }
        }

        let config_options = ConfigOptions::default();
        let node = inject_dedup_impl(node, allow_parallel);
        println!("    *** after injecting dedup (allow_parallel={allow_parallel})");
        for line in format_execution_plan(&node) {
            println!("{line}");
        }
        let node = EnforceDistribution::default()
            .optimize(node, &config_options)
            .unwrap();
        println!("    *** after enforce distribution");
        for line in format_execution_plan(&node) {
            println!("{line}");
        }
        if !allow_parallel {
            let node = EnforceSorting::default()
                .optimize(node, &config_options)
                .unwrap();
            println!("    *** after enforce sorting");
            for line in format_execution_plan(&node) {
                println!("{line}");
            }
            node
        } else {
            node
        }
    }
}

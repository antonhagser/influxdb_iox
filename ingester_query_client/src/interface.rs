//! Interface for all implementation of the client interface.
use arrow::{datatypes::SchemaRef, record_batch::RecordBatch};
use async_trait::async_trait;
use data_types::{NamespaceId, TableId, TimestampMinMax, TransitionPartitionId};
use datafusion::prelude::Expr;
use futures::Stream;
use std::fmt::Debug;
use trace::span::Span;
use uuid::Uuid;

/// Request to an ingester.
#[derive(Debug, Clone)]
pub struct QueryRequest {
    /// Namespace to search
    pub namespace_id: NamespaceId,

    /// Table that should be queried.
    pub table_id: TableId,

    /// Columns the query service is interested in.
    pub columns: Vec<String>,

    /// Predicate for filtering.
    ///
    /// The client (and ingester) SHOULD apply these but are free to drop some or all of the expressions.
    pub filters: Vec<Expr>,
}

/// Response from the ingester.
#[derive(Debug, Clone)]
pub struct QueryResponse<S>
where
    S: Send + Stream<Item = ResponseStreamResult> + 'static,
{
    /// Ingester UUID
    pub ingester_uuid: Uuid,

    /// Number of persisted parquet files for this ingester.
    pub persist_counter: i64,

    /// Serialized table schema.
    pub table_schema: SchemaRef,

    /// Ingester partitions.
    pub partitions: Vec<QueryResponsePartition>,

    /// Payload stream.
    pub payload_stream: S,
}

/// Partition metadata as part of a [`QueryResponse`].
#[derive(Debug, Clone)]
pub struct QueryResponsePartition {
    /// Partition ID.
    pub id: TransitionPartitionId,

    /// Timestamp min and max.
    pub t_min_max: TimestampMinMax,

    /// Partition schema.
    ///
    /// This is always a projection of the [table schema](QueryResponse::table_schema).
    pub schema: SchemaRef,
}

/// Data payload in the [`QueryResponse`].
#[derive(Debug, Clone)]
pub struct QueryResponsePayload {
    /// Associated partition.
    ///
    /// This is always one of the partitions present in [`QueryResponse::partitions`].
    pub partition_id: TransitionPartitionId,

    /// Record batch data.
    ///
    /// This is always a projection of the [partition schema](QueryResponsePartition::schema).
    pub batch: RecordBatch,
}

/// Query error.
pub type QueryError = Box<dyn std::error::Error + Send + Sync>;

/// Result of the response stream.
pub type ResponseStreamResult = Result<QueryResponsePayload, QueryError>;

/// Abstract query client.
#[async_trait]
pub trait IngesterQueryClient: Debug + Send + Sync + 'static {
    /// Response stream type.
    type ResponseStream: Send + Stream<Item = ResponseStreamResult> + 'static;

    /// Perform request.
    async fn query(
        &self,
        request: QueryRequest,
        span: Option<Span>,
    ) -> Result<QueryResponse<Self::ResponseStream>, QueryError>;
}

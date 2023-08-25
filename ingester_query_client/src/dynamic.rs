//! Type-erased implementations of the client interface.

use std::sync::Arc;

use async_trait::async_trait;
use futures::{stream::BoxStream, Stream, StreamExt};
use trace::span::Span;

use crate::interface::{
    IngesterQueryClient, QueryError, QueryRequest, QueryResponse, ResponseStreamResult,
};

/// Type-erased response stream.
pub type DynResponseStream = BoxStream<'static, ResponseStreamResult>;

/// Type-erased [`IngesterQueryClient`].
pub type DynIngesterQueryClient = Arc<dyn IngesterQueryClient<ResponseStream = DynResponseStream>>;

/// Erase type of [`IngesterQueryClient`].
///
/// This should normally be done after building client and apply all wrappers.
pub fn erase_client_type<C>(client: C) -> DynIngesterQueryClient
where
    C: IngesterQueryClient,
{
    Arc::new(BoxedStreamClient { inner: client })
}

#[async_trait]
impl<S> IngesterQueryClient for Arc<dyn IngesterQueryClient<ResponseStream = S>>
where
    S: Send + Stream<Item = ResponseStreamResult> + 'static,
{
    type ResponseStream = S;

    async fn query(
        &self,
        request: QueryRequest,
        span: Option<Span>,
    ) -> Result<QueryResponse<Self::ResponseStream>, QueryError> {
        self.as_ref().query(request, span).await
    }
}

/// Helper type to type-erase the [response stream](IngesterQueryClient::ResponseStream).
#[derive(Debug)]
struct BoxedStreamClient<C>
where
    C: IngesterQueryClient,
{
    inner: C,
}

#[async_trait]
impl<C> IngesterQueryClient for BoxedStreamClient<C>
where
    C: IngesterQueryClient,
{
    type ResponseStream = DynResponseStream;

    async fn query(
        &self,
        request: QueryRequest,
        span: Option<Span>,
    ) -> Result<QueryResponse<Self::ResponseStream>, QueryError> {
        let resp = self.inner.query(request, span).await?;

        Ok(QueryResponse {
            ingester_uuid: resp.ingester_uuid,
            persist_counter: resp.persist_counter,
            table_schema: resp.table_schema,
            partitions: resp.partitions,
            payload_stream: resp.payload_stream.boxed(),
        })
    }
}

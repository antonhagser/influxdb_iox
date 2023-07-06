use std::{fmt::Display, sync::Arc};

use compactor_scheduler::PartitionsSource;
use futures::{stream::BoxStream, StreamExt};

use super::PartitionStream;

#[derive(Debug)]
pub struct OncePartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    source: Arc<T>,
}

impl<T, P> OncePartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    pub fn new(source: T) -> Self {
        Self {
            source: Arc::new(source),
        }
    }
}

impl<T, P> Display for OncePartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "once({})", self.source)
    }
}

impl<T, P> PartitionStream for OncePartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    type Output = P;

    fn stream(&self) -> BoxStream<'_, Self::Output> {
        let source = Arc::clone(&self.source);
        futures::stream::once(async move { futures::stream::iter(source.fetch().await) })
            .flatten()
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use compactor_scheduler::MockPartitionsSource;
    use data_types::PartitionId;

    use super::*;

    #[test]
    fn test_display() {
        let stream = OncePartititionStream::new(MockPartitionsSource::new(vec![]));
        assert_eq!(stream.to_string(), "once(mock)");
    }

    #[tokio::test]
    async fn test_stream() {
        let ids = vec![
            PartitionId::new(1),
            PartitionId::new(3),
            PartitionId::new(2),
        ];
        let stream = OncePartititionStream::new(MockPartitionsSource::new(ids.clone()));

        // stream is stateless
        for _ in 0..2 {
            assert_eq!(stream.stream().collect::<Vec<_>>().await, ids,);
        }
    }
}

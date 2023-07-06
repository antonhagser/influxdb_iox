use std::{collections::VecDeque, fmt::Display, sync::Arc};

use compactor_scheduler::PartitionsSource;
use futures::{stream::BoxStream, StreamExt};

use super::PartitionStream;

#[derive(Debug)]
pub struct EndlessPartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    source: Arc<T>,
}

impl<T, P> EndlessPartititionStream<T, P>
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

impl<T, P> Display for EndlessPartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "endless({})", self.source)
    }
}

impl<T, P> PartitionStream for EndlessPartititionStream<T, P>
where
    T: PartitionsSource<Output = P>,
    P: std::fmt::Debug + Send,
{
    type Output = P;

    fn stream(&self) -> BoxStream<'_, Self::Output> {
        let source = Arc::clone(&self.source);

        // Note: we use a VecDeque as a buffer so we can preserve the order and cheaply remove the first element without
        // relocating the entire buffer content.
        futures::stream::unfold(VecDeque::new(), move |mut buffer| {
            let source = Arc::clone(&source);
            async move {
                loop {
                    if let Some(partition) = buffer.pop_front() {
                        return Some((partition, buffer));
                    }

                    // fetch new data
                    buffer = VecDeque::from(source.fetch().await);
                }
            }
        })
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
        let stream = EndlessPartititionStream::new(MockPartitionsSource::new(vec![]));
        assert_eq!(stream.to_string(), "endless(mock)");
    }

    #[tokio::test]
    async fn test_stream() {
        let ids = vec![
            PartitionId::new(1),
            PartitionId::new(3),
            PartitionId::new(2),
        ];
        let stream = EndlessPartititionStream::new(MockPartitionsSource::new(ids.clone()));

        // stream is stateless
        for _ in 0..2 {
            // we need to limit the stream at one point because it is endless
            assert_eq!(
                stream.stream().take(5).collect::<Vec<_>>().await,
                vec![
                    PartitionId::new(1),
                    PartitionId::new(3),
                    PartitionId::new(2),
                    PartitionId::new(1),
                    PartitionId::new(3)
                ],
            );
        }
    }
}

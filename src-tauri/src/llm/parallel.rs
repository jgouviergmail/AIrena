//! Bounded concurrency for independent LLM calls.
//!
//! The engine prepares owned requests, runs them through [`run_bounded`] with
//! the provider's `max_parallel_calls`, then applies the results on `&mut self`
//! in the original order — the same "owned context, apply after the await"
//! pattern as the democratic vote in `turn_manager`.

use std::future::Future;

use futures_util::stream::{self, StreamExt};

/// Run `futures` with at most `limit` in flight (a limit of 0 counts as 1).
/// Results come back in the input order, whatever the completion order.
pub async fn run_bounded<F, T>(futures: Vec<F>, limit: usize) -> Vec<T>
where
    F: Future<Output = T>,
{
    let limit = limit.max(1);
    stream::iter(futures).buffered(limit).collect().await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    /// Tracks how many futures are in flight at once.
    async fn tracked(in_flight: Arc<AtomicUsize>, peak: Arc<AtomicUsize>, id: usize) -> usize {
        let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(20)).await;
        in_flight.fetch_sub(1, Ordering::SeqCst);
        id
    }

    #[tokio::test]
    async fn limit_one_is_strictly_sequential_and_keeps_order() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let futs: Vec<_> = (0..4).map(|i| tracked(in_flight.clone(), peak.clone(), i)).collect();
        let out = run_bounded(futs, 1).await;
        assert_eq!(out, vec![0, 1, 2, 3]);
        assert_eq!(peak.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn limit_n_overlaps_up_to_n_and_zero_means_one() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let futs: Vec<_> = (0..6).map(|i| tracked(in_flight.clone(), peak.clone(), i)).collect();
        let out = run_bounded(futs, 3).await;
        assert_eq!(out, vec![0, 1, 2, 3, 4, 5], "order is preserved");
        assert_eq!(peak.load(Ordering::SeqCst), 3);

        let peak0 = Arc::new(AtomicUsize::new(0));
        let futs: Vec<_> = (0..2).map(|i| tracked(in_flight.clone(), peak0.clone(), i)).collect();
        run_bounded(futs, 0).await;
        assert_eq!(peak0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn empty_input_returns_empty() {
        let out: Vec<u8> = run_bounded(Vec::<std::future::Ready<u8>>::new(), 4).await;
        assert!(out.is_empty());
    }
}

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;

const MAX_ENTRIES: usize = 256;
const TTL: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct VisibilityCache {
    game_entries: HashMap<Uuid, (bool, Instant)>,
    proposal_entries: HashMap<Uuid, (bool, Instant)>,
}

impl VisibilityCache {
    pub async fn check_game<F, Fut>(&mut self, game_id: Uuid, lookup: F) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<bool>>,
    {
        Self::check(&mut self.game_entries, game_id, lookup).await
    }

    pub async fn check_proposal<F, Fut>(&mut self, proposal_id: Uuid, lookup: F) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<bool>>,
    {
        Self::check(&mut self.proposal_entries, proposal_id, lookup).await
    }

    async fn check<F, Fut>(
        entries: &mut HashMap<Uuid, (bool, Instant)>,
        id: Uuid,
        lookup: F,
    ) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<bool>>,
    {
        if let Some(&(visible, cached_at)) = entries.get(&id) {
            if cached_at.elapsed() < TTL {
                return visible;
            }
            entries.remove(&id);
        }
        match lookup().await {
            Ok(visible) => {
                if entries.len() >= MAX_ENTRIES
                    && let Some(oldest_key) = entries
                        .iter()
                        .min_by_key(|(_, (_, ts))| *ts)
                        .map(|(k, _)| *k)
                {
                    entries.remove(&oldest_key);
                }
                entries.insert(id, (visible, Instant::now()));
                visible
            }
            Err(e) => {
                tracing::warn!(%id, error = %e, "visibility lookup failed, failing closed");
                false
            }
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_lookup(
        count: Arc<AtomicUsize>,
        result: anyhow::Result<bool>,
    ) -> impl FnOnce() -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<bool>> + Send>,
    > + Send {
        move || {
            count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { result })
        }
    }

    #[tokio::test]
    async fn repeated_id_within_ttl_performs_one_lookup_positive() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut cache = VisibilityCache::default();
        let id = Uuid::new_v4();

        let r1 = cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(true)))
            .await;
        let r2 = cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(true)))
            .await;

        assert!(r1);
        assert!(r2);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn repeated_id_within_ttl_performs_one_lookup_negative() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut cache = VisibilityCache::default();
        let id = Uuid::new_v4();

        let r1 = cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(false)))
            .await;
        let r2 = cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(false)))
            .await;

        assert!(!r1);
        assert!(!r2);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn entry_past_ttl_is_re_looked_up() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut cache = VisibilityCache::default();
        let id = Uuid::new_v4();

        cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(true)))
            .await;
        assert_eq!(count.load(Ordering::SeqCst), 1);

        tokio::time::advance(Duration::from_secs(31)).await;

        cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(false)))
            .await;
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn lookup_error_yields_false_and_is_not_cached() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut cache = VisibilityCache::default();
        let id = Uuid::new_v4();

        let r1 = cache
            .check_game(
                id,
                counting_lookup(Arc::clone(&count), Err(anyhow::anyhow!("db error"))),
            )
            .await;
        assert!(!r1);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        let r2 = cache
            .check_game(id, counting_lookup(Arc::clone(&count), Ok(true)))
            .await;
        assert!(r2);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn map_stays_bounded_past_cap() {
        let mut cache = VisibilityCache::default();
        for _ in 0..(MAX_ENTRIES + 100) {
            let id = Uuid::new_v4();
            cache
                .check_game(id, move || Box::pin(async { Ok(true) }))
                .await;
        }
        assert!(cache.game_entries.len() <= MAX_ENTRIES);
    }

    #[tokio::test]
    async fn game_and_proposal_ids_do_not_alias() {
        let mut cache = VisibilityCache::default();
        let shared_id = Uuid::new_v4();

        let game_result = cache
            .check_game(shared_id, || Box::pin(async { Ok(true) }))
            .await;
        let proposal_result = cache
            .check_proposal(shared_id, || Box::pin(async { Ok(false) }))
            .await;

        assert!(game_result);
        assert!(!proposal_result);
    }
}

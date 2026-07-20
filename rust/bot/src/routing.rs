use crate::config::ProviderConfig;

#[derive(Debug)]
pub struct ProviderRouter {
    providers: Vec<ProviderConfig>,
    index: usize,
}

impl ProviderRouter {
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        Self {
            providers,
            index: 0,
        }
    }

    pub fn next(&mut self) -> Option<&ProviderConfig> {
        self.providers.get(self.index)
    }

    pub fn mark_failed(&mut self) {
        self.index = self.index.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(model: &str, priority: i32) -> ProviderConfig {
        ProviderConfig {
            url: "http://x".to_string(),
            api_key: None,
            model: model.to_string(),
            reasoning_effort: None,
            extra_body: None,
            priority,
        }
    }

    #[test]
    fn empty_router_returns_none() {
        let mut router = ProviderRouter::new(vec![]);
        assert!(router.next().is_none());
    }

    #[test]
    fn walks_providers_in_order_then_exhausts() {
        let mut router =
            ProviderRouter::new(vec![provider("a", 0), provider("b", 1), provider("c", 2)]);

        assert_eq!(router.next().unwrap().model, "a");
        router.mark_failed();
        assert_eq!(router.next().unwrap().model, "b");
        router.mark_failed();
        assert_eq!(router.next().unwrap().model, "c");
        router.mark_failed();
        assert!(router.next().is_none());
    }

    #[test]
    fn mark_failed_past_end_stays_none() {
        let mut router = ProviderRouter::new(vec![provider("a", 0)]);
        router.mark_failed();
        router.mark_failed();
        assert!(router.next().is_none());
    }
}

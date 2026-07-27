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

    pub fn current(&mut self) -> Option<&ProviderConfig> {
        self.providers.get(self.index)
    }

    pub fn fail_over(&mut self) {
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
        assert!(router.current().is_none());
    }

    #[test]
    fn walks_providers_in_order_then_exhausts() {
        let mut router =
            ProviderRouter::new(vec![provider("a", 0), provider("b", 1), provider("c", 2)]);

        assert_eq!(router.current().unwrap().model, "a");
        router.fail_over();
        assert_eq!(router.current().unwrap().model, "b");
        router.fail_over();
        assert_eq!(router.current().unwrap().model, "c");
        router.fail_over();
        assert!(router.current().is_none());
    }

    #[test]
    fn fail_over_past_end_stays_none() {
        let mut router = ProviderRouter::new(vec![provider("a", 0)]);
        router.fail_over();
        router.fail_over();
        assert!(router.current().is_none());
    }
}

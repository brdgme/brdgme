use crate::card::Resource;

pub type Cost = brdgme_cost::Cost<Resource>;

pub fn can_afford(a: &Cost, c: &Cost) -> bool {
    let mut short = 0;
    for (&g, &n) in &c.0 {
        if a.get(&g) < n {
            short += n - a.get(&g);
        }
    }
    a.get(&Resource::Gold) - c.get(&Resource::Gold) >= short
}

#[cfg(test)]
mod tests {
    use super::*;
    use Resource::*;
    use brdgme_game::Gamer;
    use std::collections::HashMap;

    #[test]
    fn test_can_afford_exact_payment_no_gold() {
        assert!(can_afford(
            &brdgme_cost::Cost(HashMap::from([(Emerald, 3)])),
            &brdgme_cost::Cost(HashMap::from([(Emerald, 3)])),
        ));
    }

    #[test]
    fn test_can_afford_gold_covers_shortfall_exactly() {
        assert!(can_afford(
            &brdgme_cost::Cost(HashMap::from([(Emerald, 2), (Gold, 1)])),
            &brdgme_cost::Cost(HashMap::from([(Emerald, 3)])),
        ));
    }

    #[test]
    fn test_can_afford_gold_one_short() {
        assert!(!can_afford(
            &brdgme_cost::Cost(HashMap::from([(Emerald, 2), (Gold, 1)])),
            &brdgme_cost::Cost(HashMap::from([(Emerald, 4)])),
        ));
    }

    #[test]
    fn test_can_afford_cost_names_gold() {
        assert!(can_afford(
            &brdgme_cost::Cost(HashMap::from([(Gold, 3)])),
            &brdgme_cost::Cost(HashMap::from([(Gold, 2)])),
        ));
        assert!(!can_afford(
            &brdgme_cost::Cost(HashMap::from([(Gold, 1)])),
            &brdgme_cost::Cost(HashMap::from([(Gold, 2)])),
        ));
    }

    #[test]
    fn test_can_afford_empty_cost() {
        assert!(can_afford(
            &brdgme_cost::Cost(HashMap::new()),
            &brdgme_cost::Cost(HashMap::new()),
        ));
        assert!(can_afford(
            &brdgme_cost::Cost(HashMap::from([(Emerald, 2)])),
            &brdgme_cost::Cost(HashMap::new()),
        ));
    }

    #[test]
    fn test_can_afford_shortfall_across_two_resources() {
        assert!(can_afford(
            &brdgme_cost::Cost(HashMap::from([(Emerald, 1), (Ruby, 1), (Gold, 2)])),
            &brdgme_cost::Cost(HashMap::from([(Emerald, 2), (Ruby, 2)])),
        ));
        assert!(!can_afford(
            &brdgme_cost::Cost(HashMap::from([(Emerald, 1), (Ruby, 1), (Gold, 1)])),
            &brdgme_cost::Cost(HashMap::from([(Emerald, 2), (Ruby, 2)])),
        ));
    }

    #[test]
    fn test_game_serde_round_trip() {
        let (game, _) = crate::Game::start(2, 42).unwrap();
        let json = serde_json::to_string(&game).unwrap();
        let restored: crate::Game = serde_json::from_str(&json).unwrap();
        assert_eq!(game, restored);
    }
}

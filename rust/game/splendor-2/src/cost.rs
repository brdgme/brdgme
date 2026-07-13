use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::card::Resource;

/// Map from resource to count. Ported from `brdgme-go/libcost/cost.go`, but
/// only the subset actually used by `splendor_1` (see the port plan's Global
/// Constraints for the full list of dropped functions: `Take`, `Drop`,
/// `Keys`, `IsZero`, `Trim`, `Ints`, `PosNeg` is kept only internally by
/// `can_afford`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost(pub HashMap<Resource, i32>);

impl Cost {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Ported from `libcost.FromInts`.
    pub fn from_resources(resources: &[Resource]) -> Self {
        let mut c = Self::new();
        for &r in resources {
            *c.0.entry(r).or_insert(0) += 1;
        }
        c
    }

    /// Get the count for a resource, defaulting to 0 if absent.
    pub fn get(&self, r: Resource) -> i32 {
        *self.0.get(&r).unwrap_or(&0)
    }

    pub fn set(&mut self, r: Resource, v: i32) {
        self.0.insert(r, v);
    }

    /// Ported from `libcost.Cost.Add`.
    pub fn add(&self, other: &Cost) -> Cost {
        let mut nc = self.clone();
        for (&k, &v) in &other.0 {
            let entry = nc.0.entry(k).or_insert(0);
            *entry += v;
        }
        nc
    }

    /// Ported from `libcost.Cost.Inv`.
    pub fn inv(&self) -> Cost {
        let mut nc = Cost::new();
        for (&k, &v) in &self.0 {
            nc.set(k, -v);
        }
        nc
    }

    /// Ported from `libcost.Cost.Sub`.
    pub fn sub(&self, other: &Cost) -> Cost {
        self.add(&other.inv())
    }

    /// Ported from `libcost.Cost.CanAfford` (a plain per-key `>=` check, no
    /// gold-shortfall logic - distinct from splendor's own `can_afford` in
    /// this module).
    pub fn can_afford(&self, other: &Cost) -> bool {
        let diff = self.sub(other);
        diff.0.values().all(|&v| v >= 0)
    }

    /// Ported from `libcost.Cost.Sum`.
    pub fn sum(&self) -> i32 {
        self.0.values().sum()
    }
}

/// Splendor's own affordability check (`amount.go`'s `CanAfford`), distinct
/// from `Cost::can_afford` above: this one folds in a gold reserve to cover
/// any per-resource shortfall.
pub fn can_afford(a: &Cost, c: &Cost) -> bool {
    let mut short = 0;
    for (&g, &n) in &c.0 {
        if a.get(g) < n {
            short += n - a.get(g);
        }
    }
    a.get(Resource::Gold) - c.get(Resource::Gold) >= short
}

#[cfg(test)]
mod tests {
    use super::*;
    use Resource::*;

    #[test]
    fn test_cost_clone() {
        let c1 = Cost(HashMap::from([(Diamond, 4), (Sapphire, 5)]));
        let mut c2 = c1.clone();
        c2.set(Diamond, 10);
        assert_eq!(Cost(HashMap::from([(Diamond, 4), (Sapphire, 5)])), c1);
    }

    #[test]
    fn test_cost_add() {
        let c1 = Cost(HashMap::from([(Diamond, 4), (Sapphire, 5)]));
        let c2 = Cost(HashMap::from([(Diamond, 3)]));
        assert_eq!(
            Cost(HashMap::from([(Diamond, 7), (Sapphire, 5)])),
            c1.add(&c2)
        );
    }

    #[test]
    fn test_cost_inv() {
        let c1 = Cost(HashMap::from([(Diamond, -3), (Sapphire, 6)]));
        assert_eq!(
            Cost(HashMap::from([(Diamond, 3), (Sapphire, -6)])),
            c1.inv()
        );
    }

    #[test]
    fn test_cost_sub() {
        let c1 = Cost(HashMap::from([(Diamond, 2), (Sapphire, 3)]));
        let c2 = Cost(HashMap::from([(Diamond, 1), (Sapphire, 4)]));
        assert_eq!(
            Cost(HashMap::from([(Diamond, 1), (Sapphire, -1)])),
            c1.sub(&c2)
        );
    }

    #[test]
    fn test_cost_can_afford() {
        let c = Cost(HashMap::from([(Diamond, 3), (Sapphire, 4)]));
        assert!(c.can_afford(&Cost(HashMap::from([(Diamond, 2), (Sapphire, 4)]))));
        assert!(!c.can_afford(&Cost(HashMap::from([(Diamond, 5), (Sapphire, 4)]))));
    }

    #[test]
    fn test_cost_sum() {
        let c = Cost(HashMap::from([(Diamond, 2), (Sapphire, 1), (Emerald, 3)]));
        assert_eq!(6, c.sum());
    }

    #[test]
    fn test_can_afford() {
        assert!(can_afford(
            &Cost(HashMap::from([(Emerald, 2), (Gold, 1)])),
            &Cost(HashMap::from([(Emerald, 3)])),
        ));
        assert!(!can_afford(
            &Cost(HashMap::from([(Emerald, 2), (Gold, 1)])),
            &Cost(HashMap::from([(Emerald, 4)])),
        ));
    }
}

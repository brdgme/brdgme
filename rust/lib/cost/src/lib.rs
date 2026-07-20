use std::collections::HashMap;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost<K: Hash + Eq>(pub HashMap<K, i32>);

impl<K: Hash + Eq> Default for Cost<K> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<K: Hash + Eq + Clone> Cost<K> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_keys(keys: impl IntoIterator<Item = K>) -> Self {
        let mut map = HashMap::new();
        for k in keys {
            *map.entry(k).or_insert(0) += 1;
        }
        Self(map)
    }

    #[must_use]
    pub fn add(&self, other: &Cost<K>) -> Cost<K> {
        let mut map = self.0.clone();
        for (k, v) in &other.0 {
            *map.entry(k.clone()).or_insert(0) += v;
        }
        Cost(map)
    }

    #[must_use]
    pub fn inv(&self) -> Cost<K> {
        Cost(self.0.iter().map(|(k, v)| (k.clone(), -v)).collect())
    }

    #[must_use]
    pub fn sub(&self, other: &Cost<K>) -> Cost<K> {
        self.add(&other.inv())
    }

    #[must_use]
    pub fn pos_neg(&self) -> (Cost<K>, Cost<K>) {
        let mut pos = HashMap::new();
        let mut neg = HashMap::new();
        for (k, v) in &self.0 {
            if *v > 0 {
                pos.insert(k.clone(), *v);
            } else if *v < 0 {
                neg.insert(k.clone(), *v);
            }
        }
        (Cost(pos), Cost(neg))
    }

    #[must_use]
    pub fn can_afford(&self, other: &Cost<K>) -> bool {
        let (_, neg) = self.sub(other).pos_neg();
        neg.0.is_empty()
    }

    #[must_use]
    pub fn take(&self, keys: &[K]) -> Cost<K> {
        let mut map = HashMap::new();
        for k in keys {
            if let Some(v) = self.0.get(k) {
                map.insert(k.clone(), *v);
            }
        }
        Cost(map)
    }

    // The Go implementation had a bug: `for k := range keys` iterated over
    // slice indices instead of values, so it dropped keys matching the
    // indices 0..len(keys) rather than the actual key values. Fixed here.
    #[must_use]
    pub fn drop(&self, keys: &[K]) -> Cost<K> {
        let drop_set: std::collections::HashSet<&K> = keys.iter().collect();
        Cost(
            self.0
                .iter()
                .filter(|(k, _)| !drop_set.contains(k))
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
        )
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.values().all(|v| *v == 0)
    }

    #[must_use]
    pub fn trim(&self) -> Cost<K> {
        Cost(
            self.0
                .iter()
                .filter(|(_, v)| **v != 0)
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
        )
    }

    #[must_use]
    pub fn sum(&self) -> i32 {
        self.0.values().sum()
    }

    fn keys_unsorted(&self) -> Vec<K> {
        self.0
            .iter()
            .filter(|(_, v)| **v != 0)
            .map(|(k, _)| k.clone())
            .collect()
    }
}

impl<K: Hash + Eq + Ord + Clone> Cost<K> {
    #[must_use]
    pub fn keys(&self) -> Vec<K> {
        let mut keys: Vec<K> = self
            .0
            .iter()
            .filter(|(_, v)| **v != 0)
            .map(|(k, _)| k.clone())
            .collect();
        keys.sort();
        keys
    }

    #[must_use]
    pub fn to_keys(&self) -> Vec<K> {
        let mut keys: Vec<K> = Vec::new();
        for (k, v) in &self.0 {
            for _ in 0..*v {
                keys.push(k.clone());
            }
        }
        keys.sort();
        keys
    }
}

fn prepend_to_cost_arrays<K: Hash + Eq + Clone>(
    c: &Cost<K>,
    arr: &[Vec<Cost<K>>],
) -> Vec<Vec<Cost<K>>> {
    arr.iter()
        .map(|a| {
            let mut v = vec![c.clone()];
            v.extend(a.iter().cloned());
            v
        })
        .collect()
}

#[must_use]
pub fn can_afford_perm<K: Hash + Eq + Clone>(
    c: &Cost<K>,
    with: &[Vec<Cost<K>>],
) -> (bool, Vec<Vec<Cost<K>>>) {
    if c.is_zero() {
        return (true, vec![]);
    }
    if with.is_empty() {
        return (false, vec![]);
    }

    let mut can = false;
    let mut can_with: Vec<Vec<Cost<K>>> = vec![];
    let mut relevant = false;
    let c_keys = c.keys_unsorted();

    for w in &with[0] {
        if w.can_afford(c) {
            return (true, vec![vec![c.clone()]]);
        }
        let needed = w.take(&c_keys).trim();
        if needed.0.is_empty() {
            continue;
        }
        relevant = true;
        let (remaining, extra) = c.sub(&needed).pos_neg();
        let (sub_can, sub_can_with) = can_afford_perm(&remaining, &with[1..]);
        if sub_can {
            can = true;
            can_with.extend(prepend_to_cost_arrays(&needed.add(&extra), &sub_can_with));
        }
    }
    if !relevant {
        let (sub_can, sub_can_with) = can_afford_perm(c, &with[1..]);
        can = sub_can;
        can_with = prepend_to_cost_arrays(&Cost::new(), &sub_can_with);
    }
    (can, can_with)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[allow(dead_code)]
    enum TestRes {
        Res1,
        Res2,
        Res3,
        Res4,
        Res5,
    }

    fn cost(entries: &[(TestRes, i32)]) -> Cost<TestRes> {
        Cost(entries.iter().cloned().collect())
    }

    #[test]
    fn test_clone() {
        let c1 = cost(&[(TestRes::Res1, 4), (TestRes::Res2, 5)]);
        let mut c2 = c1.clone();
        c2.0.insert(TestRes::Res1, 10);
        assert_eq!(cost(&[(TestRes::Res1, 4), (TestRes::Res2, 5)]), c1);
    }

    #[test]
    fn test_add() {
        let c1 = cost(&[(TestRes::Res1, 4), (TestRes::Res2, 5)]);
        let c2 = cost(&[(TestRes::Res1, 3)]);
        assert_eq!(cost(&[(TestRes::Res1, 7), (TestRes::Res2, 5)]), c1.add(&c2));
    }

    #[test]
    fn test_inv() {
        let c1 = cost(&[(TestRes::Res1, -3), (TestRes::Res2, 6)]);
        assert_eq!(cost(&[(TestRes::Res1, 3), (TestRes::Res2, -6)]), c1.inv());
    }

    #[test]
    fn test_sub() {
        let c1 = cost(&[(TestRes::Res1, 2), (TestRes::Res2, 3)]);
        let c2 = cost(&[(TestRes::Res1, 1), (TestRes::Res2, 4)]);
        assert_eq!(
            cost(&[(TestRes::Res1, 1), (TestRes::Res2, -1)]),
            c1.sub(&c2)
        );
    }

    #[test]
    fn test_pos_neg() {
        let c = cost(&[(TestRes::Res1, 4), (TestRes::Res2, -5)]);
        let (pos, neg) = c.pos_neg();
        assert_eq!(cost(&[(TestRes::Res1, 4)]), pos);
        assert_eq!(cost(&[(TestRes::Res2, -5)]), neg);
    }

    #[test]
    fn test_can_afford() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        assert!(c.can_afford(&cost(&[(TestRes::Res1, 2), (TestRes::Res2, 4)])));
        assert!(!c.can_afford(&cost(&[(TestRes::Res1, 5), (TestRes::Res2, 4)])));
    }

    #[test]
    fn test_take() {
        let c = cost(&[(TestRes::Res1, 4), (TestRes::Res2, 5)]);
        assert_eq!(cost(&[(TestRes::Res1, 4)]), c.take(&[TestRes::Res1]));
    }

    #[test]
    fn test_drop() {
        let c = cost(&[(TestRes::Res1, 4), (TestRes::Res2, 5)]);
        assert_eq!(cost(&[(TestRes::Res2, 5)]), c.drop(&[TestRes::Res1]));
    }

    #[test]
    fn test_keys() {
        let c = cost(&[(TestRes::Res1, 2), (TestRes::Res2, 0), (TestRes::Res3, 3)]);
        assert_eq!(vec![TestRes::Res1, TestRes::Res3], c.keys());
    }

    #[test]
    fn test_is_zero() {
        assert!(Cost::<TestRes>::new().is_zero());
        assert!(cost(&[(TestRes::Res1, 0)]).is_zero());
        assert!(!cost(&[(TestRes::Res1, 1)]).is_zero());
    }

    #[test]
    fn test_trim() {
        let c = cost(&[(TestRes::Res1, 0), (TestRes::Res2, 5)]);
        assert_eq!(cost(&[(TestRes::Res2, 5)]), c.trim());
    }

    #[test]
    fn test_to_keys() {
        let c = cost(&[(TestRes::Res1, 2), (TestRes::Res2, 1), (TestRes::Res3, 3)]);
        assert_eq!(
            vec![
                TestRes::Res1,
                TestRes::Res1,
                TestRes::Res2,
                TestRes::Res3,
                TestRes::Res3,
                TestRes::Res3,
            ],
            c.to_keys()
        );
    }

    #[test]
    fn test_sum() {
        let c = cost(&[(TestRes::Res1, 2), (TestRes::Res2, 1), (TestRes::Res3, 3)]);
        assert_eq!(6, c.sum());
    }

    #[test]
    fn test_from_keys() {
        let c = Cost::from_keys(vec![TestRes::Res1, TestRes::Res1, TestRes::Res2]);
        assert_eq!(cost(&[(TestRes::Res1, 2), (TestRes::Res2, 1)]), c);
    }

    fn deep_trim(costs: &[Cost<TestRes>]) -> Vec<Cost<TestRes>> {
        costs.iter().map(|c| c.trim()).collect()
    }

    fn double_deep_trim(costs: &[Vec<Cost<TestRes>>]) -> Vec<Vec<Cost<TestRes>>> {
        costs.iter().map(|c| deep_trim(c)).collect()
    }

    #[test]
    fn test_can_afford_perm_nothing() {
        let (can, can_with) = can_afford_perm(&Cost::new(), &[]);
        assert!(can);
        assert_eq!(Vec::<Vec<Cost<TestRes>>>::new(), can_with);
    }

    #[test]
    fn test_cant_afford_perm_nothing() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let (can, can_with) = can_afford_perm(&c, &[]);
        assert!(!can);
        assert_eq!(Vec::<Vec<Cost<TestRes>>>::new(), can_with);
    }

    #[test]
    fn test_can_afford_perm_single() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let with = vec![vec![cost(&[(TestRes::Res1, 5), (TestRes::Res2, 6)])]];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(can);
        let expected = vec![vec![cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)])]];
        assert_eq!(double_deep_trim(&expected), double_deep_trim(&can_with));
    }

    #[test]
    fn test_cant_afford_perm_single() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let with = vec![vec![cost(&[(TestRes::Res1, 2), (TestRes::Res2, 5)])]];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(!can);
        assert_eq!(Vec::<Vec<Cost<TestRes>>>::new(), can_with);
    }

    #[test]
    fn test_can_afford_perm_multiple() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let with = vec![
            vec![cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)])],
            vec![cost(&[(TestRes::Res1, 1), (TestRes::Res2, 3)])],
            vec![cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)])],
        ];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(can);
        let expected = vec![vec![
            cost(&[(TestRes::Res1, 3), (TestRes::Res2, 0)]),
            cost(&[(TestRes::Res1, 0), (TestRes::Res2, 3)]),
            cost(&[(TestRes::Res1, 0), (TestRes::Res2, 1)]),
        ]];
        assert_eq!(double_deep_trim(&expected), double_deep_trim(&can_with));
    }

    #[test]
    fn test_cant_afford_perm_multiple() {
        let c = cost(&[(TestRes::Res1, 8), (TestRes::Res2, 4)]);
        let with = vec![
            vec![cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)])],
            vec![cost(&[(TestRes::Res1, 1), (TestRes::Res2, 3)])],
            vec![cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)])],
        ];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(!can);
        assert_eq!(Vec::<Vec<Cost<TestRes>>>::new(), can_with);
    }

    #[test]
    fn test_can_afford_perm_perm() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let with = vec![vec![
            cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)]),
            cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            cost(&[(TestRes::Res1, 5), (TestRes::Res2, 6)]),
        ]];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(can);
        let expected = vec![vec![cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)])]];
        assert_eq!(double_deep_trim(&expected), double_deep_trim(&can_with));
    }

    #[test]
    fn test_cant_afford_perm_perm() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let with = vec![vec![
            cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)]),
            cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            cost(&[(TestRes::Res1, 5), (TestRes::Res2, 2)]),
        ]];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(!can);
        assert_eq!(Vec::<Vec<Cost<TestRes>>>::new(), can_with);
    }

    #[test]
    fn test_can_afford_perm_multiple_perm() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 4)]);
        let with = vec![
            vec![
                cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)]),
                cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            ],
            vec![
                cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)]),
                cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            ],
        ];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(can);
        let expected = vec![
            vec![
                cost(&[(TestRes::Res1, 3), (TestRes::Res2, 0)]),
                cost(&[(TestRes::Res1, 0), (TestRes::Res2, 4)]),
            ],
            vec![
                cost(&[(TestRes::Res1, 1), (TestRes::Res2, 4)]),
                cost(&[(TestRes::Res1, 2), (TestRes::Res2, 0)]),
            ],
        ];
        assert_eq!(double_deep_trim(&expected), double_deep_trim(&can_with));
    }

    #[test]
    fn test_cant_afford_perm_multiple_perm() {
        let c = cost(&[(TestRes::Res1, 6), (TestRes::Res2, 7)]);
        let with = vec![
            vec![
                cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)]),
                cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            ],
            vec![
                cost(&[(TestRes::Res1, 5), (TestRes::Res2, 0)]),
                cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            ],
        ];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(!can);
        assert_eq!(Vec::<Vec<Cost<TestRes>>>::new(), can_with);
    }

    #[test]
    fn test_can_afford_perm_multiple_perm_some_irrelevant() {
        let c = cost(&[(TestRes::Res1, 3), (TestRes::Res2, 1)]);
        let with = vec![
            vec![cost(&[(TestRes::Res3, 5)]), cost(&[(TestRes::Res4, 1)])],
            vec![
                cost(&[(TestRes::Res1, 5), (TestRes::Res2, 2)]),
                cost(&[(TestRes::Res1, 1), (TestRes::Res2, 5)]),
            ],
        ];
        let (can, can_with) = can_afford_perm(&c, &with);
        assert!(can);
        let expected = vec![vec![
            cost(&[(TestRes::Res1, 0), (TestRes::Res2, 0)]),
            cost(&[(TestRes::Res1, 3), (TestRes::Res2, 1)]),
        ]];
        assert_eq!(double_deep_trim(&expected), double_deep_trim(&can_with));
    }
}

//! The single source of randomness for game mechanics.
//!
//! Every game stores a [`GameRng`] in its serialized state, seeded from the
//! `seed` passed to `Gamer::start`. All shuffles, dice rolls, and random
//! selections draw from that field - never from `rand::rng()` or any other
//! ambient source. Because the RNG state is part of game state, the same
//! seed and command sequence always reproduce the same game, across process
//! restarts and save/load cycles.
//!
//! `ChaCha8Rng` is used because rust-random guarantees its output stream is
//! portable and stable across crate versions (unlike `StdRng`/`SmallRng`,
//! which explicitly are not), and it serializes its full stream position.
//!
//! Portability note: avoid sampling `usize` ranges where cross-platform
//! reproducibility matters - `usize` sampling is word-size dependent. All
//! current targets are 64-bit, so in-repo games sample `usize` freely.

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// A deterministic, serializable RNG owned by game state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRng(ChaCha8Rng);

impl GameRng {
    /// Seed deterministically; same seed = same stream, forever.
    pub fn seed_from_u64(seed: u64) -> Self {
        GameRng(ChaCha8Rng::seed_from_u64(seed))
    }

    /// Seed from OS entropy. Production path when no seed is supplied, and
    /// the serde default used as a migration shim for pre-seed game states.
    pub fn from_entropy() -> Self {
        GameRng(ChaCha8Rng::from_rng(&mut rand::rng()))
    }
}

/// Only so `#[derive(Default)]` game structs compile; `start()` must always
/// overwrite the field with a properly seeded value.
impl Default for GameRng {
    fn default() -> Self {
        GameRng::seed_from_u64(0)
    }
}

impl rand::TryRng for GameRng {
    type Error = std::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.fill_bytes(dst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let draw = |seed| -> Vec<u8> {
            let mut r = GameRng::seed_from_u64(seed);
            (0..16).map(|_| r.random_range(0..100)).collect()
        };
        assert_eq!(draw(7), draw(7));
        assert_ne!(draw(7), draw(8));
    }

    #[test]
    fn serde_roundtrip_resumes_stream() {
        let mut r = GameRng::seed_from_u64(42);
        let _: u32 = r.random_range(0..100);
        let json = serde_json::to_string(&r).unwrap();
        let mut r2: GameRng = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
        assert_eq!(r.random_range(0..100u32), r2.random_range(0..100u32));
    }

    #[test]
    fn rand_ext_apis_work() {
        let mut r = GameRng::seed_from_u64(1);
        let mut v = [1, 2, 3, 4, 5];
        v.shuffle(&mut r);
        assert!(v.choose(&mut r).is_some());
    }
}

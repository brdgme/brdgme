//! Injectable randomness for game mechanics that roll dice (or draw from any
//! other fixed set of discrete outcomes).
//!
//! Production code always uses [`RngRandomizer`], which draws a real uniformly
//! random outcome - identical behavior to calling `rand::rng()` directly.
//! Tests that need dice outcomes to matter to an assertion should use
//! `brdgme_game::test_support::ScriptedRandomizer` (behind the `test-support`
//! feature) to script an exact sequence of outcomes instead of leaving the
//! result to chance. Tests where the outcome is incidental (fully overwritten
//! by direct state construction afterward, or exercising a fixed `Gamer`
//! trait entry point like `start()`/`command()` that cannot take an
//! injected randomizer) should keep using `RngRandomizer` and assert only on
//! RNG-outcome-invariant properties (counts, deterministic log content,
//! structural equality) - see `zombie-dice-2` for a worked example of all
//! three cases.
//!
//! # Design boundary
//!
//! `Randomizer` abstracts *game-meaningful outcomes* - dice rolls, shuffled
//! deck order, tile draws - NOT `rand`'s primitive API. Production
//! implementations use `rand` internally; tests script the outcomes
//! directly (e.g. the exact post-shuffle order of a deck). The trait
//! intentionally does not mirror `rand`'s trait surface: scripting
//! primitives like `gen_range` makes test doubles depend on algorithm
//! internals (e.g. how many draws a Fisher-Yates shuffle consumes) and is
//! brittle. Input-taking operations beyond what `zombie-dice-2` needs
//! ("shuffle this deck", "roll N dice") are deliberately out of scope for
//! now - extend the pattern, at the outcome level, when a real consumer
//! (e.g. a card game) adopts it. For genuinely distribution-heavy needs,
//! the escape hatch is a test-local `Randomizer` impl over a seeded
//! `RngCore`, accepting that test's local seed-brittleness.
use rand::prelude::*;

/// A source of "which outcome did this roll show" decisions, given the list
/// of possible outcomes for a single roll.
pub trait Randomizer<T> {
    /// Return one of `faces` for a single roll. Implementations may ignore
    /// `faces` entirely (e.g. a scripted test double that always returns a
    /// pre-chosen value regardless of what was actually rolled).
    fn next(&mut self, faces: &[T]) -> T;
}

/// Production randomizer: draws a real uniformly random outcome from `faces`.
#[derive(Default)]
pub struct RngRandomizer;

impl<T: Copy> Randomizer<T> for RngRandomizer {
    fn next(&mut self, faces: &[T]) -> T {
        faces[rand::rng().random_range(0..faces.len())]
    }
}

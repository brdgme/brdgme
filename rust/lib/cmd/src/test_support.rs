//! Generic contract regression harness for `Gamer` implementations.
//!
//! Enabled via the `test-support` feature so it is never compiled into
//! release builds. Game crates depend on it as a `dev-dependency` and call
//! `assert_gamer_contract::<Game>()` from a test.

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use brdgme_game::Gamer;

use crate::api::{Request, Response};
use crate::requester::Requester;
use crate::requester::gamer;

/// Drives a `Gamer` implementation through the full request/response
/// contract and asserts the invariants every game is expected to uphold.
pub fn assert_gamer_contract<G>()
where
    G: Gamer + Debug + Clone + Serialize + DeserializeOwned,
{
    let mut requester = gamer::new::<G>();

    let player_counts = match requester.request(&Request::PlayerCounts).unwrap() {
        Response::PlayerCounts { player_counts } => player_counts,
        r => panic!("expected PlayerCounts response, got {:?}", r),
    };
    assert!(!player_counts.is_empty(), "player_counts must not be empty");

    match requester.request(&Request::Rules).unwrap() {
        Response::Rules { rules } => {
            assert!(!rules.trim().is_empty(), "rules must not be empty")
        }
        r => panic!("expected Rules response, got {:?}", r),
    }

    let max_count = *player_counts.iter().max().unwrap();
    let unadvertised_count = (0..=max_count + 1)
        .find(|c| !player_counts.contains(c))
        .expect("there must be an unadvertised player count between 0 and max + 1");
    match requester
        .request(&Request::New {
            players: unadvertised_count,
        })
        .unwrap()
    {
        Response::UserError { .. } | Response::SystemError { .. } => {}
        r => panic!(
            "expected New with unadvertised player count {} to fail, got {:?}",
            unadvertised_count, r
        ),
    }

    for &count in &player_counts {
        let (game_state, status) =
            match requester.request(&Request::New { players: count }).unwrap() {
                Response::New {
                    game,
                    public_render,
                    player_renders,
                    ..
                } => {
                    assert_eq!(
                        player_renders.len(),
                        count,
                        "New: player_renders length should match player count {}",
                        count
                    );
                    assert!(
                        !public_render.render.trim().is_empty(),
                        "New: public render must not be empty for player count {}",
                        count
                    );
                    for player_render in &player_renders {
                        assert!(
                            !player_render.render.trim().is_empty(),
                            "New: player render must not be empty for player count {}",
                            count
                        );
                    }
                    (game.state, game.status)
                }
                r => panic!(
                    "expected New to succeed for advertised player count {}, got {:?}",
                    count, r
                ),
            };

        match requester
            .request(&Request::Status {
                game: game_state.clone(),
            })
            .unwrap()
        {
            Response::Status {
                game,
                public_render,
                player_renders,
            } => {
                assert_eq!(
                    game.status, status,
                    "Status: status should round-trip through serialize/deserialize for player count {}",
                    count
                );
                assert_eq!(
                    player_renders.len(),
                    count,
                    "Status: player_renders length should match player count {}",
                    count
                );
                assert!(
                    !public_render.render.trim().is_empty(),
                    "Status: public render must not be empty for player count {}",
                    count
                );
                for player_render in &player_renders {
                    assert!(
                        !player_render.render.trim().is_empty(),
                        "Status: player render must not be empty for player count {}",
                        count
                    );
                }
            }
            r => panic!(
                "expected Status to succeed for player count {}, got {:?}",
                count, r
            ),
        }

        match requester
            .request(&Request::Play {
                player: 0,
                command: "!!! not a valid command @@@ ###".to_string(),
                names: (0..count).map(|i| format!("player{}", i)).collect(),
                game: game_state,
            })
            .unwrap()
        {
            Response::UserError { .. } => {}
            r => panic!(
                "expected Play with garbage input to return UserError (never SystemError) for player count {}, got {:?}",
                count, r
            ),
        }
    }
}

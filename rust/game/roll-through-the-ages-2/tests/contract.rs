use brdgme_cmd::test_support::assert_gamer_contract;
use roll_through_the_ages_2::Game;

// Game/command()/render are placeholder stubs until Task 2/3/4 land; the
// contract exercises real command flow and render output, so it's ignored
// here and un-ignored once those tasks wire up the full Gamer impl.
#[test]
#[ignore]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

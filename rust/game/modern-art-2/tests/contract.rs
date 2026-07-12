use brdgme_cmd::test_support::assert_gamer_contract;
use modern_art_2::Game;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

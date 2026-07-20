use alhambra_1::Game;
use brdgme_cmd::test_support::assert_gamer_contract;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

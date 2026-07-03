use brdgme_cmd::test_support::assert_gamer_contract;
use lords_of_vegas_1::Game;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

use age_of_war_2::Game;
use brdgme_cmd::test_support::assert_gamer_contract;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

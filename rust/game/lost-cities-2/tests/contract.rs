use brdgme_cmd::test_support::assert_gamer_contract;
use lost_cities_2::Game;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

use brdgme_cmd::test_support::assert_gamer_contract;
use starship_catan_1::Game;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

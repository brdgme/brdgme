use brdgme_cmd::test_support::assert_gamer_contract;
use category_5_2::Game;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

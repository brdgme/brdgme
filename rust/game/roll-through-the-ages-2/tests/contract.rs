use brdgme_cmd::test_support::assert_gamer_contract;
use roll_through_the_ages_2::Game;

#[test]
fn game_contract() {
    assert_gamer_contract::<Game>();
}

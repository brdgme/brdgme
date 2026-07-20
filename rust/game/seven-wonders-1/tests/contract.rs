use brdgme_cmd::test_support::assert_gamer_contract;
use seven_wonders_1::Game;

#[test]
fn gamer_contract() {
    assert_gamer_contract::<Game>();
}

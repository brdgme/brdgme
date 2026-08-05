#[test]
fn rating_before_aggregates_exclude_nulls() {
    assert_eq!(rating_before(0), 42);
}

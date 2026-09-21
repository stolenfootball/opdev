use opdev_test_strength_fixture::allowed;

#[test]
fn capacity() {
    assert!(allowed(9));
    assert!(!allowed(11));
    // Deliberately weak: the canary adds assert!(!allowed(10)).
}

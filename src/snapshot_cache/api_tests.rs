use super::*;

#[test]
fn api_try_read_distinguishes_contention_and_never_acquires() {
    let (_owner, reader) = new("never-opened-api-fixture", Duration::from_secs(1)).unwrap();
    let held = reader.shared.state.write().unwrap();
    assert!(reader.try_read().unwrap().is_none());
    drop(held);
    let mut view = reader.try_read().unwrap().unwrap();
    assert!(matches!(
        view.assess().unwrap().latest,
        LatestAttempt::Empty
    ));
}

#[test]
fn api_try_read_does_not_hide_publication_poison() {
    let (_owner, reader) = new("never-opened-api-fixture", Duration::from_secs(1)).unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = reader.shared.state.write().unwrap();
        panic!("injected publication poison");
    }));
    assert_eq!(reader.try_read().err(), Some(CacheError::Poisoned));
}

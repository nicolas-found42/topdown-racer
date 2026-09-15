use topdown_racer::records::RecordBook;

#[test]
fn eligible_records_persist_separately_by_identity() {
    let path = std::env::temp_dir().join(format!("racer-records-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut book = RecordBook::load(&path);
    assert!(!book.consider("race:a:raw", 15.0, false));
    assert!(book.consider("race:a:raw", 30.0, true));
    assert!(!book.consider("race:a:raw", f32::NAN, true));
    assert!(!book.consider("race:a:raw", 31.0, true));
    book.consider("practice:a:raw", 8.0, true);
    book.save(&path).unwrap();
    let loaded = RecordBook::load(&path);
    assert_eq!(loaded.best("race:a:raw"), Some(30.0));
    assert_eq!(loaded.best("practice:a:raw"), Some(8.0));
    assert_eq!(loaded.best("race:b:raw"), None);
    assert_eq!(loaded.best("race:a:smooth"), None);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn malformed_records_and_failed_writes_keep_an_eligible_session_best() {
    let root = std::env::temp_dir().join(format!("racer-record-failure-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("malformed.json");
    std::fs::write(&path, "not json").unwrap();
    let mut book = RecordBook::load(&path);
    assert!(book.consider("race:current", 20.0, true));
    assert!(
        book.save(&root).is_err(),
        "a directory cannot be replaced by the record file"
    );
    assert_eq!(book.best("race:current"), Some(20.0));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

/// Record identity is content derived: the same authored Track yields the same
/// identity, a different Track yields a different race key, and a book saved
/// under one configuration is never consulted for another. The handling-version
/// and eligibility-policy fields of `race_key` are constants today, so the
/// steering response is the policy dimension that actually varies a key.
#[test]
fn incompatible_configurations_keep_separate_records() {
    use topdown_racer::controls::SteeringResponse;
    use topdown_racer::records::{race_key, track_identity};
    use topdown_racer_core::track::{Track, HILLSIDE_CIRCUIT, SAMPLE_CIRCUIT};

    let sample = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let sample_again = Track::parse(SAMPLE_CIRCUIT).unwrap();
    let hillside = Track::parse(HILLSIDE_CIRCUIT).unwrap();

    // Stable across parses of the same authored content...
    assert_eq!(track_identity(&sample), track_identity(&sample_again));
    // ...and distinct for genuinely different geometry.
    assert_ne!(track_identity(&sample), track_identity(&hillside));

    let key_a = race_key(&sample, SteeringResponse::Raw);
    let key_other_track = race_key(&hillside, SteeringResponse::Raw);
    let key_other_steering = race_key(&sample, SteeringResponse::Smooth);
    assert_ne!(key_a, key_other_track);
    assert_ne!(key_a, key_other_steering);

    let path = std::env::temp_dir().join(format!(
        "racer-records-identity-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut book = RecordBook::load(&path);
    assert!(book.consider(&key_a, 42.0, true));
    book.save(&path).unwrap();
    let loaded = RecordBook::load(&path);
    assert_eq!(loaded.best(&key_a), Some(42.0));
    assert_eq!(loaded.best(&key_other_track), None);
    assert_eq!(loaded.best(&key_other_steering), None);
    std::fs::remove_file(path).unwrap();
}

/// Only an eligible lap may create or replace a record: an ineligible lap is
/// ignored even when it is faster than the stored one.
#[test]
fn an_ineligible_update_cannot_overwrite_or_create_a_record() {
    let mut book = RecordBook::default();
    assert!(!book.consider("race:key", 12.0, false));
    assert_eq!(book.best("race:key"), None);
    assert!(book.consider("race:key", 30.0, true));
    assert!(!book.consider("race:key", 20.0, false));
    assert_eq!(book.best("race:key"), Some(30.0));
}

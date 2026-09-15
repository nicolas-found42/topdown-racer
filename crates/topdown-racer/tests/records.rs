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

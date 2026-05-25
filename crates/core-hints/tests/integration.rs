use core_hints::{HintEngine, HintSource, Platform};
use core_persistence::Database;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn open_tmp_db() -> Database {
    let dir = tempdir_inproc();
    Database::open(&dir.join("test.sqlite")).expect("db opens")
}

// Tiny temp dir helper to avoid adding a dev-dependency.
fn tempdir_inproc() -> std::path::PathBuf {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let nano = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = base.join(format!("zapx-test-{pid}-{nano}-{seq}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn record_and_suggest_history() {
    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    engine.record(None, "ls -la").unwrap();
    engine.record(None, "ls -lh").unwrap();
    engine.record(None, "git status").unwrap();

    let hints = engine.suggest(None, Platform::Generic, "ls", 10).unwrap();
    assert_eq!(hints.len(), 2);
    assert!(hints.iter().all(|h| h.text.starts_with("ls")));
    assert!(hints.iter().all(|h| matches!(h.source, HintSource::History)));
}

#[test]
fn sensitive_not_recorded() {
    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    engine.record(None, "password: hunter2").unwrap();
    engine.record(None, "export API_KEY=abc").unwrap();
    let hints = engine.suggest(None, Platform::Generic, "p", 10).unwrap();
    assert!(hints.is_empty(), "sensitive commands must not enter history");
}

#[test]
fn catalog_fills_when_history_empty() {
    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    let hints = engine.suggest(None, Platform::Linux, "system", 5).unwrap();
    assert!(!hints.is_empty());
    assert!(hints.iter().all(|h| h.text.starts_with("system")));
    assert!(matches!(hints[0].source, HintSource::Catalog { platform: Platform::Linux }));
}

#[test]
fn snippet_outranks_history() {
    let db = open_tmp_db();
    // main's snippets table is (name, content) — global, not session-scoped.
    db.create_snippet("list", "ls -lah --color").unwrap();
    let engine = HintEngine::new(&db);
    engine.record(None, "ls -la").unwrap();
    engine.record(None, "ls -la").unwrap(); // bump freq

    let hints = engine.suggest(None, Platform::Generic, "ls", 5).unwrap();
    assert!(matches!(hints[0].source, HintSource::Snippet));
    assert_eq!(hints[0].text, "ls -lah --color");
    assert_eq!(hints[0].label.as_deref(), Some("list"));
}

#[test]
fn dedupe_keeps_higher_source() {
    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    // The catalog already has "ls -la"; if the user also typed it, history
    // should win since it scores higher.
    engine.record(None, "ls -la").unwrap();
    let hints = engine.suggest(None, Platform::Linux, "ls -la", 5).unwrap();
    let ls = hints.iter().find(|h| h.text == "ls -la").unwrap();
    assert!(matches!(ls.source, HintSource::History));
}

#[test]
fn empty_prefix_returns_empty() {
    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    engine.record(None, "anything").unwrap();
    let hints = engine.suggest(None, Platform::Linux, "", 5).unwrap();
    assert!(hints.is_empty());
}

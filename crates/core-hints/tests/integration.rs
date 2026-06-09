use core_hints::detector::PlatformDetector;
use core_hints::{catalog, HintEngine, HintSource, Platform};
use core_persistence::Database;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
    db.create_snippet("list", "ls -lah --color", None).unwrap();
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

// ---------------------------------------------------------------------------
// End-to-end: detect a platform from output, then ask the engine for hints
// against THAT platform. Mirrors the real session flow (PTY bytes → detector
// → catalog → suggestions). Serialised with `CATALOG_LOCK` because
// `catalog::init` / `catalog::reload` mutate a single process-wide registry.
// ---------------------------------------------------------------------------

/// `catalog::init` and `catalog::reload` mutate process-global state. The
/// `cargo test` harness runs integration tests in parallel by default, so
/// any test that calls those functions must hold this lock for the duration
/// of the test to avoid a flaky race.
static CATALOG_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn detect_then_suggest_pipeline_fortigate() {
    let _g = CATALOG_LOCK.lock().unwrap();
    // No user dir — only the bundled catalogs are in play.
    catalog::init(None);

    // Feed a realistic FortiGate login transcript byte by byte; the detector
    // should latch onto FortiGate before the prompt finishes.
    let mut det = PlatformDetector::new();
    for chunk in [
        b"FortiGate-100D login: admin\n".as_ref(),
        b"Password: \n",
        b"Welcome!\n",
        b"fgt-edge # ".as_ref(),
    ] {
        let _ = det.feed(chunk);
    }
    let platform = det.detected().expect("detector should latch on FortiGate");
    assert_eq!(platform, Platform::Fortigate);

    // Now ask the engine for hints — without any history, the bundled
    // FortiGate catalog should populate them.
    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    let hints = engine.suggest(None, platform, "diagnose sniffer", 5).unwrap();
    assert!(!hints.is_empty(), "FortiGate catalog must include diagnose sniffer commands");
    assert!(
        hints.iter().any(|h| h.text.contains("diagnose sniffer packet")),
        "expected `diagnose sniffer packet` in suggestions, got: {:?}",
        hints.iter().map(|h| &h.text).collect::<Vec<_>>()
    );
    for h in &hints {
        assert!(matches!(h.source, HintSource::Catalog { platform: Platform::Fortigate }));
    }
}

#[test]
fn user_catalog_override_replaces_bundled_for_one_platform() {
    let _g = CATALOG_LOCK.lock().unwrap();
    let user_dir = tempdir_inproc().join("catalogs");
    std::fs::create_dir_all(&user_dir).unwrap();
    // A deliberately tiny override that does NOT contain anything matching
    // "diagnose sniffer" — proves the override fully replaces (not merges).
    let payload = r#"["just-my-one-command"]"#;
    std::fs::write(user_dir.join("fortigate.json"), payload).unwrap();

    catalog::init(Some(user_dir.clone()));
    let n_applied = catalog::reload().expect("reload should succeed");
    assert_eq!(n_applied, 1, "exactly one override file present");

    let db = open_tmp_db();
    let engine = HintEngine::new(&db);
    // Bundled would have many `diagnose sniffer` entries; the override has none.
    let hints = engine
        .suggest(None, Platform::Fortigate, "diagnose sniffer", 5)
        .unwrap();
    assert!(hints.is_empty(), "override replaces, not merges");

    // But the user-supplied entry IS reachable.
    let hits = engine.suggest(None, Platform::Fortigate, "just-my", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].text, "just-my-one-command");

    // Reset for subsequent tests so we don't leak the override.
    catalog::init(None);
}

#[test]
fn detector_does_not_latch_on_generic_router_hash() {
    // A reproduction of the bug we fixed: a bare `router1#` with no banner
    // in the buffer must NOT auto-classify as Linux (or anything else).
    let mut det = PlatformDetector::new();
    det.feed(b"\n\nrouter1#");
    assert_eq!(det.detected(), None);
}

#[test]
fn reload_with_invalid_json_surfaces_error() {
    let _g = CATALOG_LOCK.lock().unwrap();
    let user_dir = tempdir_inproc().join("catalogs");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("linux.json"), "{ not valid json").unwrap();

    catalog::init(Some(user_dir.clone()));
    let err = catalog::reload().expect_err("invalid json must surface as Error");
    let msg = err.to_string();
    assert!(
        msg.contains("linux"),
        "error must name the offending file, got: {msg}"
    );

    // Reset so unrelated tests aren't affected.
    catalog::init(None);
}

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use gc_core::models::SessionSummary;
use gc_core::parser::{self, ParseError};
use gc_core::store::Store;
use uuid::Uuid;

const SESSION_ID: &str = "00000000-0000-4000-8000-000000000001";
const BASE: &str = include_str!("fixtures/claude/base.jsonl");
const APPEND: &str = include_str!("fixtures/claude/append.jsonl");
const UNKNOWN: &str = include_str!("fixtures/claude/unknown-entry.jsonl");

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("gc-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct EnvSnapshot {
    home: Option<OsString>,
    claude_config_dir: Option<OsString>,
}

impl EnvSnapshot {
    fn capture() -> Self {
        Self {
            home: std::env::var_os("HOME"),
            claude_config_dir: std::env::var_os("CLAUDE_CONFIG_DIR"),
        }
    }
}

impl Drop for EnvSnapshot {
    fn drop(&mut self) {
        restore_env("HOME", self.home.take());
        restore_env("CLAUDE_CONFIG_DIR", self.claude_config_dir.take());
    }
}

fn restore_env(key: &str, value: Option<OsString>) {
    match value {
        Some(value) => unsafe { std::env::set_var(key, value) },
        None => unsafe { std::env::remove_var(key) },
    }
}

fn transcript_path(root: &TestDir, name: &str) -> PathBuf {
    root.path().join(name)
}

fn write_transcript(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write synthetic transcript");
}

fn summary(session_id: Uuid, project_path: &str) -> SessionSummary {
    SessionSummary {
        session_id,
        project_path: project_path.to_string(),
        display_name: project_path
            .rsplit('/')
            .next()
            .unwrap_or(project_path)
            .to_string(),
        custom_title: None,
        ai_title: None,
        agent_name: None,
        started_at: None,
        updated_at: None,
        version: None,
        git_branch: None,
        kind: None,
        status: None,
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_cache_read_tokens: 0,
        total_cache_creation_tokens: 0,
        message_count: 0,
    }
}

#[test]
#[ignore = "GC-33: incremental watcher must reload the persisted summary"]
fn incremental_append_preserves_persisted_totals() {
    let root = TestDir::new("incremental-totals");
    let path = transcript_path(&root, &format!("{SESSION_ID}.jsonl"));
    write_transcript(&path, BASE);

    let first = parser::parse_session_incremental("/repo", &path, 0, None).unwrap();
    fs::write(&path, format!("{BASE}{APPEND}")).unwrap();

    // Mirrors the current watcher call, which reads the cursor but not the persisted summary.
    let update = parser::parse_session_incremental("/repo", &path, first.new_offset, None).unwrap();

    assert_eq!(update.summary.total_input_tokens, 15);
    assert_eq!(update.summary.total_output_tokens, 3);
    assert_eq!(update.summary.message_count, 3);
}

#[test]
#[ignore = "GC-33: cursor may advance only through newline-terminated bytes"]
fn partial_jsonl_record_does_not_advance_the_cursor() {
    let root = TestDir::new("partial-record");
    let path = transcript_path(&root, &format!("{SESSION_ID}.jsonl"));
    let partial = r#"{"type":"assistant","message":{"role":"assistant""#;
    write_transcript(&path, &format!("{BASE}{partial}"));

    let result = parser::parse_session_incremental("/repo", &path, 0, None).unwrap();

    assert_eq!(result.new_offset, BASE.len() as u64);
    assert_eq!(result.summary.message_count, 2);
}

#[test]
#[ignore = "GC-33: truncation must reset parser state"]
fn truncation_resets_cursor_and_summary_to_file_contents() {
    let root = TestDir::new("truncation");
    let path = transcript_path(&root, &format!("{SESSION_ID}.jsonl"));
    write_transcript(&path, &format!("{BASE}{APPEND}"));
    let first = parser::parse_session_incremental("/repo", &path, 0, None).unwrap();

    write_transcript(&path, BASE);
    let result =
        parser::parse_session_incremental("/repo", &path, first.new_offset, Some(first.summary))
            .unwrap();

    assert_eq!(result.new_offset, BASE.len() as u64);
    assert_eq!(result.summary.total_input_tokens, 10);
    assert_eq!(result.summary.total_output_tokens, 2);
    assert_eq!(result.summary.total_cache_read_tokens, 4);
    assert_eq!(result.summary.total_cache_creation_tokens, 3);
    assert_eq!(result.summary.message_count, 2);
}

#[test]
#[ignore = "GC-33: ambiguous hyphenated paths must remain explicitly unresolved"]
fn hyphenated_project_directory_is_not_invented_by_replacement() {
    let root = TestDir::new("hyphenated-project");
    let encoded = "-Users-example-dev-my-project";
    fs::create_dir(root.path().join(encoded)).unwrap();

    let projects = parser::list_projects(root.path()).unwrap();

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].encoded_path, encoded);
    assert_eq!(projects[0].original_path, encoded);
}

#[test]
#[ignore = "GC-33: configuration must honor CLAUDE_CONFIG_DIR and avoid panics"]
fn claude_directory_configuration_is_explicit_and_non_panicking() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _snapshot = EnvSnapshot::capture();
    let configured = TestDir::new("claude-config");

    unsafe {
        std::env::set_var("CLAUDE_CONFIG_DIR", configured.path());
        std::env::set_var("HOME", "/synthetic/home");
    }
    assert_eq!(gc_core::claude_home(), configured.path());

    unsafe {
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::remove_var("HOME");
    }
    let result = std::panic::catch_unwind(gc_core::claude_home);
    assert!(result.is_ok(), "missing HOME must not panic");
}

#[test]
#[ignore = "GC-33: unknown variants require stable diagnostics"]
fn unknown_entry_is_not_silently_omitted() {
    let root = TestDir::new("unknown-entry");
    let path = transcript_path(&root, &format!("{SESSION_ID}.jsonl"));
    write_transcript(&path, UNKNOWN);

    let result = parser::parse_session_summary("/repo", &path);

    assert!(matches!(result, Err(ParseError::Json { .. })));
}

#[test]
#[ignore = "GC-33: invalid filenames require deterministic source identity"]
fn invalid_session_filename_never_creates_a_random_identity() {
    let root = TestDir::new("invalid-session-id");
    let path = transcript_path(&root, "not-a-uuid.jsonl");
    write_transcript(&path, BASE);

    let first = parser::parse_session_summary("/repo", &path).unwrap();
    let second = parser::parse_session_summary("/repo", &path).unwrap();

    assert_eq!(first.session_id, second.session_id);
}

#[test]
#[ignore = "GC-33: project filtering must happen in SQLite before pagination"]
fn project_filter_can_find_a_session_beyond_the_first_page() {
    let store = Store::open_in_memory().unwrap();
    let target_id = Uuid::new_v4();
    store
        .upsert_session(&summary(target_id, "/projects/target"))
        .unwrap();

    thread::sleep(Duration::from_millis(1_100));
    for index in 0..100 {
        store
            .upsert_session(&summary(
                Uuid::new_v4(),
                &format!("/projects/other-{index}"),
            ))
            .unwrap();
    }

    let filtered: Vec<_> = store
        .all_sessions()
        .unwrap()
        .into_iter()
        .filter(|row| row.project_path.contains("target"))
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].session_id, target_id.to_string());
}

#[test]
#[ignore = "GC-33: full rebuild must reconcile deleted transcripts"]
fn rebuild_removes_a_deleted_transcript_from_the_read_model() {
    let root = TestDir::new("stale-deletion");
    let project_dir = root.path().join("project");
    fs::create_dir(&project_dir).unwrap();
    let path = project_dir.join(format!("{SESSION_ID}.jsonl"));
    write_transcript(&path, BASE);
    let store = Store::open_in_memory().unwrap();

    for transcript in parser::list_session_jsonls(&project_dir).unwrap() {
        let parsed = parser::parse_session_summary("/repo", &transcript).unwrap();
        store.upsert_session(&parsed).unwrap();
    }
    fs::remove_file(path).unwrap();
    for transcript in parser::list_session_jsonls(&project_dir).unwrap() {
        let parsed = parser::parse_session_summary("/repo", &transcript).unwrap();
        store.upsert_session(&parsed).unwrap();
    }

    assert!(store.all_sessions().unwrap().is_empty());
}

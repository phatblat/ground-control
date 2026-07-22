use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gc_core::parser::{self, ParseError};
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
#[ignore = "GC-33: unknown variants must report their line and discriminator"]
fn unknown_entry_reports_stable_source_diagnostic() {
    let root = TestDir::new("unknown-entry");
    let path = transcript_path(&root, &format!("{SESSION_ID}.jsonl"));
    write_transcript(&path, UNKNOWN);

    let result = parser::parse_session_summary("/repo", &path);

    assert!(matches!(
        result,
        Err(ParseError::Json { line: 1, ref source })
            if source.to_string().contains("future-provider-event")
    ));
}

#[test]
#[ignore = "GC-33: invalid filenames must use transcript identity"]
fn invalid_session_filename_uses_transcript_identity() {
    let root = TestDir::new("invalid-session-id");
    let path = transcript_path(&root, "not-a-uuid.jsonl");
    write_transcript(&path, BASE);

    let parsed = parser::parse_session_summary("/repo", &path).unwrap();
    let transcript_id = Uuid::parse_str(SESSION_ID).unwrap();

    assert_eq!(parsed.session_id, transcript_id);
}

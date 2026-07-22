use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use clap::{Parser, Subcommand};
use gc_core::{parser, projects_dir, sessions_dir, store::Store, watcher::GcWatcher};

#[derive(Parser)]
#[command(
    name = "gc",
    about = "Ground Control — monitor and manage Claude Code sessions"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List sessions across all projects
    List {
        /// Filter by project name
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Search sessions by title, agent name, or project
    Search {
        /// Search query
        query: String,
    },
    /// Show token usage summary
    Burn,
    /// Show currently live sessions
    Live,
    /// Reindex all session data
    Index,
    /// Watch live sessions with auto-refresh
    Watch,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db_path = db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = Store::open(&db_path)?;

    match cli.command {
        Commands::List { project } => cmd_list(&store, project.as_deref())?,
        Commands::Search { query } => cmd_search(&store, &query)?,
        Commands::Burn => cmd_burn(&store)?,
        Commands::Live => cmd_live()?,
        Commands::Index => cmd_index(&store)?,
        Commands::Watch => cmd_watch(&store)?,
    }

    Ok(())
}

fn cmd_list(store: &Store, project_filter: Option<&str>) -> anyhow::Result<()> {
    let results = list_sessions(store, project_filter)?;

    if results.is_empty() {
        println!("No sessions found. Run `gc index` to build the index.");
        return Ok(());
    }

    println!(
        "{:<40} {:<20} {:>10} {:>8}",
        "TITLE", "PROJECT", "TOKENS", "MSGS"
    );
    println!("{}", "-".repeat(82));
    for r in &results {
        let tokens = r.total_tokens();
        println!(
            "{:<40} {:<20} {:>10} {:>8}",
            truncate(r.title(), 39),
            truncate(&r.display_name, 19),
            format_tokens(tokens),
            r.message_count,
        );
    }
    println!("\n{} session(s)", results.len());
    Ok(())
}

fn list_sessions(
    store: &Store,
    project_filter: Option<&str>,
) -> anyhow::Result<Vec<gc_core::store::SearchResult>> {
    let results = store.all_sessions()?;
    let results: Vec<_> = match project_filter {
        Some(filter) => results
            .into_iter()
            .filter(|r| r.display_name.contains(filter) || r.project_path.contains(filter))
            .collect(),
        None => results,
    };
    Ok(results)
}

fn cmd_search(store: &Store, query: &str) -> anyhow::Result<()> {
    let results = store.search(query)?;
    if results.is_empty() {
        println!("No results for '{query}'.");
        return Ok(());
    }

    println!("{:<40} {:<20} {:>10}", "TITLE", "PROJECT", "TOKENS");
    println!("{}", "-".repeat(74));
    for r in &results {
        let tokens = r.total_tokens();
        println!(
            "{:<40} {:<20} {:>10}",
            truncate(r.title(), 39),
            truncate(&r.display_name, 19),
            format_tokens(tokens),
        );
    }
    println!("\n{} result(s)", results.len());
    Ok(())
}

fn cmd_burn(store: &Store) -> anyhow::Result<()> {
    let summary = store.token_summary()?;
    let total_input = summary.total_input + summary.total_cache_read + summary.total_cache_create;
    let total = total_input + summary.total_output;

    println!("Ground Control — Token Burn Summary");
    println!("{}", "=".repeat(44));
    println!("Sessions:           {:>12}", summary.session_count);
    println!("Messages:           {:>12}", summary.total_messages);
    println!("{}", "-".repeat(44));
    println!(
        "Input (non-cached): {:>12}",
        format_tokens(summary.total_input)
    );
    println!(
        "Input (cache read): {:>12}",
        format_tokens(summary.total_cache_read)
    );
    println!(
        "Input (cache new):  {:>12}",
        format_tokens(summary.total_cache_create)
    );
    println!("Input total:        {:>12}", format_tokens(total_input));
    println!("{}", "-".repeat(44));
    println!(
        "Output:             {:>12}",
        format_tokens(summary.total_output)
    );
    println!("{}", "=".repeat(44));
    println!("Total:              {:>12}", format_tokens(total));
    Ok(())
}

fn cmd_live() -> anyhow::Result<()> {
    let sessions = parser::list_live_sessions(&sessions_dir())?;
    print_live_table(&sessions);
    Ok(())
}

fn print_live_table(sessions: &[gc_core::models::SessionRegistry]) {
    if sessions.is_empty() {
        println!("No live sessions.");
        return;
    }

    println!("{:<8} {:<12} {:<30} {:<10}", "PID", "STATUS", "CWD", "NAME");
    println!("{}", "-".repeat(64));
    for s in sessions {
        let cwd_short = s.cwd.rsplit('/').next().unwrap_or(&s.cwd);
        println!(
            "{:<8} {:<12} {:<30} {:<10}",
            s.pid,
            format!("{:?}", s.status).to_lowercase(),
            truncate(cwd_short, 29),
            s.name.as_deref().unwrap_or("-"),
        );
    }
    println!("\n{} live session(s)", sessions.len());
}

fn cmd_watch(store: &Store) -> anyhow::Result<()> {
    let sessions_dir = sessions_dir();
    let projects_dir = projects_dir();

    let watcher = GcWatcher::new(&sessions_dir, &projects_dir)
        .map_err(|e| anyhow::anyhow!("Failed to start file watcher: {e}"))?;

    // Initial display
    let sessions = parser::list_live_sessions(&sessions_dir).unwrap_or_default();
    print_live_table(&sessions);
    print_refresh_timestamp();

    loop {
        match watcher.recv_timeout(Duration::from_secs(2)) {
            Ok(gc_core::watcher::GcEvent::SessionFileChanged {
                project_path,
                jsonl_path,
            }) => {
                let _ = index_changed_session(store, &project_path, &jsonl_path);

                refresh_display(&sessions_dir);
            }
            Ok(_event) => {
                // Session started/updated/ended — refresh the live table
                refresh_display(&sessions_dir);
            }
            Err(RecvTimeoutError::Timeout) => {
                // Periodic refresh to catch any missed changes
                refresh_display(&sessions_dir);
            }
            Err(RecvTimeoutError::Disconnected) => {
                // Watcher dropped — exit gracefully
                break;
            }
        }
    }

    Ok(())
}

fn index_changed_session(
    store: &Store,
    project_path: &str,
    jsonl_path: &Path,
) -> anyhow::Result<()> {
    let jsonl_key = jsonl_path.to_string_lossy().to_string();
    let offset = store.get_byte_offset(&jsonl_key)?;
    let result = parser::parse_session_incremental(project_path, jsonl_path, offset, None)?;
    store.upsert_session(&result.summary)?;
    store.set_byte_offset(&jsonl_key, result.new_offset)?;
    Ok(())
}

fn refresh_display(sessions_dir: &std::path::Path) {
    // Clear screen and move cursor to top-left
    print!("\x1b[2J\x1b[H");
    let sessions = parser::list_live_sessions(sessions_dir).unwrap_or_default();
    print_live_table(&sessions);
    print_refresh_timestamp();
}

fn print_refresh_timestamp() {
    let now = chrono::Local::now();
    println!("\nLast refresh: {}", now.format("%Y-%m-%d %H:%M:%S"));
}

fn cmd_index(store: &Store) -> anyhow::Result<()> {
    let projects_dir = projects_dir();
    let (total, project_count) = rebuild_index(store, &projects_dir)?;

    println!("Indexed {total} sessions across {project_count} projects.");
    Ok(())
}

fn rebuild_index(store: &Store, projects_dir: &Path) -> anyhow::Result<(usize, usize)> {
    let projects = parser::list_projects(projects_dir)?;
    let mut total = 0;

    for project in &projects {
        let project_dir = projects_dir.join(&project.encoded_path);
        let jsonls = parser::list_session_jsonls(&project_dir)?;

        for jsonl_path in &jsonls {
            match parser::parse_session_summary(&project.original_path, jsonl_path) {
                Ok(summary) => {
                    store.upsert_session(&summary)?;
                    total += 1;
                }
                Err(e) => {
                    eprintln!(
                        "warn: skipping {}: {}",
                        jsonl_path.file_name().unwrap_or_default().to_string_lossy(),
                        e
                    );
                }
            }
        }
    }

    Ok((total, projects.len()))
}

fn db_path() -> PathBuf {
    let data_dir = dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ground-control");
    data_dir.join("index.db")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

fn format_tokens(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use uuid::Uuid;

    use super::*;
    use gc_core::models::SessionSummary;

    const SESSION_ID: &str = "00000000-0000-4000-8000-000000000001";
    const BASE: &str = include_str!("../../gc-core/tests/fixtures/claude/base.jsonl");
    const APPEND: &str = include_str!("../../gc-core/tests/fixtures/claude/append.jsonl");

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
    fn incremental_watch_update_preserves_persisted_totals() {
        let root = TestDir::new("incremental-totals");
        let path = root.path().join(format!("{SESSION_ID}.jsonl"));
        write_transcript(&path, BASE);
        let store = Store::open_in_memory().unwrap();

        index_changed_session(&store, "/repo", &path).unwrap();
        fs::write(&path, format!("{BASE}{APPEND}")).unwrap();
        index_changed_session(&store, "/repo", &path).unwrap();

        let sessions = store.all_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].input_tokens, 15);
        assert_eq!(sessions[0].output_tokens, 3);
        assert_eq!(sessions[0].message_count, 3);
    }

    #[test]
    #[ignore = "GC-33: project filtering must happen in SQLite before pagination"]
    fn project_filter_can_find_a_session_beyond_the_first_page() {
        let store = Store::open_in_memory().unwrap();
        let target_id = Uuid::new_v4();
        store
            .upsert_session(&summary(target_id, "/projects/target"))
            .unwrap();

        std::thread::sleep(Duration::from_millis(1_100));
        for index in 0..100 {
            store
                .upsert_session(&summary(
                    Uuid::new_v4(),
                    &format!("/projects/other-{index}"),
                ))
                .unwrap();
        }

        let filtered = list_sessions(&store, Some("target")).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].session_id, target_id.to_string());
    }

    #[test]
    #[ignore = "GC-33: full rebuild must reconcile deleted transcripts"]
    fn rebuild_removes_a_deleted_transcript_from_the_read_model() {
        let root = TestDir::new("stale-deletion");
        let project_dir = root.path().join("-repo");
        fs::create_dir(&project_dir).unwrap();
        let path = project_dir.join(format!("{SESSION_ID}.jsonl"));
        write_transcript(&path, BASE);
        let store = Store::open_in_memory().unwrap();

        rebuild_index(&store, root.path()).unwrap();
        fs::remove_file(path).unwrap();
        rebuild_index(&store, root.path()).unwrap();

        assert!(store.all_sessions().unwrap().is_empty());
    }
}

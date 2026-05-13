use clap::{Parser, Subcommand};
use gc_core::{parser, projects_dir, sessions_dir, store::Store};
use std::path::PathBuf;

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
    }

    Ok(())
}

fn cmd_list(store: &Store, project_filter: Option<&str>) -> anyhow::Result<()> {
    let results = store.all_sessions()?;
    let results: Vec<_> = match project_filter {
        Some(filter) => results
            .into_iter()
            .filter(|r| r.display_name.contains(filter) || r.project_path.contains(filter))
            .collect(),
        None => results,
    };

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
    let total_input =
        summary.total_input + summary.total_cache_read + summary.total_cache_create;
    let total = total_input + summary.total_output;

    println!("Ground Control — Token Burn Summary");
    println!("{}", "=".repeat(44));
    println!("Sessions:           {:>12}", summary.session_count);
    println!("Messages:           {:>12}", summary.total_messages);
    println!("{}", "-".repeat(44));
    println!("Input (non-cached): {:>12}", format_tokens(summary.total_input));
    println!("Input (cache read): {:>12}", format_tokens(summary.total_cache_read));
    println!("Input (cache new):  {:>12}", format_tokens(summary.total_cache_create));
    println!("Input total:        {:>12}", format_tokens(total_input));
    println!("{}", "-".repeat(44));
    println!("Output:             {:>12}", format_tokens(summary.total_output));
    println!("{}", "=".repeat(44));
    println!("Total:              {:>12}", format_tokens(total));
    Ok(())
}

fn cmd_live() -> anyhow::Result<()> {
    let sessions = parser::list_live_sessions(&sessions_dir())?;
    if sessions.is_empty() {
        println!("No live sessions.");
        return Ok(());
    }

    println!("{:<8} {:<12} {:<30} {:<10}", "PID", "STATUS", "CWD", "NAME");
    println!("{}", "-".repeat(64));
    for s in &sessions {
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
    Ok(())
}

fn cmd_index(store: &Store) -> anyhow::Result<()> {
    let projects_dir = projects_dir();
    let projects = parser::list_projects(&projects_dir)?;
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

    println!(
        "Indexed {total} sessions across {} projects.",
        projects.len()
    );
    Ok(())
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

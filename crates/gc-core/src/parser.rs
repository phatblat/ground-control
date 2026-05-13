use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::models::*;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error at line {line}: {source}")]
    Json { line: usize, source: serde_json::Error },
}

pub fn read_session_registry(path: &Path) -> Result<SessionRegistry, ParseError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(|e| ParseError::Json { line: 1, source: e })
}

pub fn list_live_sessions(sessions_dir: &Path) -> Result<Vec<SessionRegistry>, ParseError> {
    let mut sessions = Vec::new();
    if !sessions_dir.exists() {
        return Ok(sessions);
    }
    for entry in fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            match read_session_registry(&path) {
                Ok(reg) => sessions.push(reg),
                Err(_) => continue,
            }
        }
    }
    Ok(sessions)
}

pub fn list_projects(projects_dir: &Path) -> Result<Vec<ProjectInfo>, ParseError> {
    let mut projects = Vec::new();
    if !projects_dir.exists() {
        return Ok(projects);
    }
    for entry in fs::read_dir(projects_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let encoded = entry.file_name().to_string_lossy().to_string();
        let original = decode_project_path(&encoded);
        let display = original
            .rsplit('/')
            .next()
            .unwrap_or(&original)
            .to_string();
        let session_count = fs::read_dir(entry.path())?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "jsonl")
            })
            .count();
        projects.push(ProjectInfo {
            encoded_path: encoded,
            original_path: original,
            display_name: display,
            session_count,
        });
    }
    projects.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(projects)
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub encoded_path: String,
    pub original_path: String,
    pub display_name: String,
    pub session_count: usize,
}

pub fn parse_session_summary(
    project_path: &str,
    jsonl_path: &Path,
) -> Result<SessionSummary, ParseError> {
    let session_id = jsonl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .unwrap_or_else(uuid::Uuid::new_v4);

    let file = fs::File::open(jsonl_path)?;
    let reader = BufReader::new(file);

    let mut summary = SessionSummary {
        session_id,
        project_path: project_path.to_string(),
        display_name: String::new(),
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
    };

    for (_line_num, line) in reader.lines().enumerate() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let entry: SessionEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        match entry {
            SessionEntry::CustomTitle(t) => summary.custom_title = Some(t.custom_title),
            SessionEntry::AiTitle(t) => summary.ai_title = Some(t.ai_title),
            SessionEntry::AgentName(a) => summary.agent_name = Some(a.agent_name),
            SessionEntry::Assistant(a) => {
                summary.message_count += 1;
                if summary.version.is_none() {
                    summary.version = a.common.version.clone();
                }
                if summary.git_branch.is_none() {
                    summary.git_branch = a.common.git_branch.clone();
                }
                if let Some(usage) = &a.message.usage {
                    summary.total_input_tokens += usage.input_tokens.unwrap_or(0);
                    summary.total_output_tokens += usage.output_tokens.unwrap_or(0);
                    summary.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                    summary.total_cache_creation_tokens +=
                        usage.cache_creation_input_tokens.unwrap_or(0);
                }
            }
            SessionEntry::User(u) => {
                summary.message_count += 1;
                if summary.version.is_none() {
                    summary.version = u.common.version.clone();
                }
            }
            _ => {}
        }
    }

    summary.display_name = project_path
        .rsplit('/')
        .next()
        .unwrap_or(project_path)
        .to_string();

    Ok(summary)
}

pub fn list_session_jsonls(project_dir: &Path) -> Result<Vec<PathBuf>, ParseError> {
    let mut paths = Vec::new();
    if !project_dir.exists() {
        return Ok(paths);
    }
    for entry in fs::read_dir(project_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn decode_project_path(encoded: &str) -> String {
    encoded.replace('-', "/")
}

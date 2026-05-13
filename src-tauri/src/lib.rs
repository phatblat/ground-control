use gc_core::{parser, projects_dir, sessions_dir};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LiveSession {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub status: String,
    pub name: Option<String>,
    pub version: String,
    pub kind: String,
}

#[tauri::command]
fn list_live_sessions() -> Result<Vec<LiveSession>, String> {
    let sessions = parser::list_live_sessions(&sessions_dir()).map_err(|e| e.to_string())?;
    Ok(sessions
        .into_iter()
        .map(|s| LiveSession {
            pid: s.pid,
            session_id: s.session_id.to_string(),
            cwd: s.cwd,
            status: format!("{:?}", s.status).to_lowercase(),
            name: s.name,
            version: s.version,
            kind: format!("{:?}", s.kind).to_lowercase(),
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct Project {
    pub encoded_path: String,
    pub original_path: String,
    pub display_name: String,
    pub session_count: usize,
}

#[tauri::command]
fn list_projects() -> Result<Vec<Project>, String> {
    let projects = parser::list_projects(&projects_dir()).map_err(|e| e.to_string())?;
    Ok(projects
        .into_iter()
        .map(|p| Project {
            encoded_path: p.encoded_path,
            original_path: p.original_path,
            display_name: p.display_name,
            session_count: p.session_count,
        })
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![list_live_sessions, list_projects])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

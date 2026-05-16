use std::time::Duration;

use gc_core::watcher::GcWatcher;
use gc_core::{parser, projects_dir, sessions_dir};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct LiveSession {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    pub status: String,
    pub name: Option<String>,
    pub version: String,
    pub kind: String,
}

fn poll_sessions() -> Vec<LiveSession> {
    parser::list_live_sessions(&sessions_dir())
        .unwrap_or_default()
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
        .collect()
}

#[tauri::command]
fn list_live_sessions() -> Result<Vec<LiveSession>, String> {
    Ok(poll_sessions())
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

pub fn update_tray_title(app: &tauri::AppHandle, count: usize) {
    if let Some(tray) = app.tray_by_id("main") {
        let label = format!("GC: {} session{}", count, if count == 1 { "" } else { "s" });
        let _ = tray.set_tooltip(Some(&label));
        let _ = tray.set_title(Some(&label));
    }
}

fn start_watcher(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let sessions_path = sessions_dir();
        let projects_path = projects_dir();

        let watcher = match GcWatcher::new(&sessions_path, &projects_path) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create watcher: {e}");
                return;
            }
        };

        // Emit initial session state
        let sessions = poll_sessions();
        update_tray_title(&app_handle, sessions.len());
        let _ = app_handle.emit("sessions-updated", &sessions);

        loop {
            match watcher.recv_timeout(Duration::from_secs(2)) {
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    let sessions = poll_sessions();
                    update_tray_title(&app_handle, sessions.len());
                    let _ = app_handle.emit("sessions-updated", &sessions);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    eprintln!("Watcher channel disconnected");
                    break;
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", "Show Ground Control", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;

            let count = parser::list_live_sessions(&sessions_dir())
                .map(|s| s.len())
                .unwrap_or(0);
            let initial_label =
                format!("GC: {} session{}", count, if count == 1 { "" } else { "s" });

            TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip(&initial_label)
                .title(&initial_label)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            start_watcher(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_live_sessions, list_projects])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

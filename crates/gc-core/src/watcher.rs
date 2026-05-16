use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::models::SessionRegistry;
use crate::parser;

#[derive(Debug, Clone)]
pub enum GcEvent {
    SessionStarted(SessionRegistry),
    SessionUpdated(SessionRegistry),
    SessionEnded {
        pid: u32,
        session_id: String,
    },
    SessionFileChanged {
        project_path: String,
        jsonl_path: PathBuf,
    },
    Error(String),
}

pub struct GcWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<GcEvent>,
}

impl GcWatcher {
    pub fn new(sessions_dir: &Path, projects_dir: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();
        let sessions_dir_owned = sessions_dir.to_path_buf();
        let projects_dir_owned = projects_dir.to_path_buf();

        let event_tx = tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    let events = classify_event(&event, &sessions_dir_owned, &projects_dir_owned);
                    for gc_event in events {
                        let _ = event_tx.send(gc_event);
                    }
                }
                Err(e) => {
                    let _ = event_tx.send(GcEvent::Error(e.to_string()));
                }
            })?;

        if sessions_dir.exists() {
            watcher.watch(sessions_dir, RecursiveMode::NonRecursive)?;
        }
        if projects_dir.exists() {
            watcher.watch(projects_dir, RecursiveMode::Recursive)?;
        }

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn recv(&self) -> Result<GcEvent, mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn try_recv(&self) -> Result<GcEvent, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<GcEvent, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

fn classify_event(event: &Event, sessions_dir: &Path, projects_dir: &Path) -> Vec<GcEvent> {
    let mut results = Vec::new();

    for path in &event.paths {
        if path.starts_with(sessions_dir) && path.extension().is_some_and(|e| e == "json") {
            match &event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    match parser::read_session_registry(path) {
                        Ok(reg) => {
                            if matches!(event.kind, EventKind::Create(_)) {
                                results.push(GcEvent::SessionStarted(reg));
                            } else {
                                results.push(GcEvent::SessionUpdated(reg));
                            }
                        }
                        Err(e) => {
                            log::debug!(
                                "Failed to read session registry {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
                EventKind::Remove(_) => {
                    let pid = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0);
                    results.push(GcEvent::SessionEnded {
                        pid,
                        session_id: String::new(),
                    });
                }
                _ => {}
            }
        } else if path.starts_with(projects_dir)
            && path.extension().is_some_and(|e| e == "jsonl")
            && matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_))
        {
            let project_path = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|encoded| encoded.replace('-', "/"))
                .unwrap_or_default();

            results.push(GcEvent::SessionFileChanged {
                project_path,
                jsonl_path: path.clone(),
            });
        }
    }

    results
}

pub fn poll_live_sessions(sessions_dir: &Path) -> Vec<SessionRegistry> {
    parser::list_live_sessions(sessions_dir).unwrap_or_default()
}

pub fn detect_status_changes(old: &[SessionRegistry], new: &[SessionRegistry]) -> Vec<GcEvent> {
    let mut events = Vec::new();

    for n in new {
        match old.iter().find(|o| o.pid == n.pid) {
            Some(o) if o.status != n.status => {
                events.push(GcEvent::SessionUpdated(n.clone()));
            }
            None => {
                events.push(GcEvent::SessionStarted(n.clone()));
            }
            _ => {}
        }
    }

    for o in old {
        if !new.iter().any(|n| n.pid == o.pid) {
            events.push(GcEvent::SessionEnded {
                pid: o.pid,
                session_id: o.session_id.to_string(),
            });
        }
    }

    events
}

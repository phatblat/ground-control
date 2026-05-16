pub mod models;
pub mod parser;
pub mod store;
pub mod watcher;

use std::path::PathBuf;

pub fn claude_home() -> PathBuf {
    dirs().0
}

pub fn sessions_dir() -> PathBuf {
    dirs().0.join("sessions")
}

pub fn projects_dir() -> PathBuf {
    dirs().0.join("projects")
}

fn dirs() -> (PathBuf,) {
    let home = std::env::var("HOME").expect("HOME not set");
    (PathBuf::from(home).join(".claude"),)
}

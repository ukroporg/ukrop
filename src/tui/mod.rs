mod app;
pub mod config_dialog;
mod fuzzy;
mod input;
pub mod theme;
mod tty_reader;
mod ui;

use anyhow::Result;

pub use app::PickerMode;

#[derive(Clone)]
pub struct PickerEntry {
    pub display: String,
    pub value: String,
    /// For SSH hosts: full connection args (e.g. "root@host" or "-p 2222 admin@host").
    /// Used for output instead of `value` (which is the DB key).
    pub connect_value: Option<String>,
    pub score: f64,
    pub is_favorite: bool,
    pub last_time: i64,
    pub use_count: i64,
    pub exists: Option<bool>,
    pub duration_ms: Option<i64>,
}

impl PickerEntry {
    pub fn from_dirs(entries: Vec<crate::db::model::DirEntry>) -> Vec<Self> {
        entries
            .into_iter()
            .map(|e| {
                let exists = Some(std::path::Path::new(&e.path).is_dir());
                PickerEntry {
                    display: crate::util::short_path(&e.path),
                    value: e.path,
                    connect_value: None,
                    score: e.score,
                    is_favorite: e.is_favorite,
                    last_time: e.last_visit,
                    use_count: e.visit_count,
                    exists,
                    duration_ms: None,
                }
            })
            .collect()
    }

    pub fn from_cmds(entries: Vec<crate::db::model::CmdEntry>) -> Vec<Self> {
        entries
            .into_iter()
            .map(|e| {
                let duration_ms = e.duration_ms;
                PickerEntry {
                    display: e.command.clone(),
                    value: e.command,
                    connect_value: None,
                    score: e.score,
                    is_favorite: e.is_favorite,
                    last_time: e.last_used,
                    use_count: e.use_count,
                    exists: None,
                    duration_ms,
                }
            })
            .collect()
    }

    pub fn from_ssh_hosts(entries: Vec<crate::db::model::SshHostEntry>) -> Vec<Self> {
        entries
            .into_iter()
            .map(|e| {
                let display = format_ssh_display(&e);
                let hostname = e.hostname.as_deref().unwrap_or(&e.host);
                let connect = crate::history::format_ssh_args(
                    e.user.as_deref(),
                    hostname,
                    e.port,
                );
                PickerEntry {
                    display,
                    connect_value: Some(connect),
                    value: e.host,
                    score: e.score,
                    is_favorite: e.is_favorite,
                    last_time: e.last_used,
                    use_count: e.use_count,
                    exists: None,
                    duration_ms: None,
                }
            })
            .collect()
    }
}

fn format_ssh_display(e: &crate::db::model::SshHostEntry) -> String {
    let mut detail = String::new();
    if let Some(ref u) = e.user {
        detail.push_str(u);
        detail.push('@');
    }
    if let Some(ref h) = e.hostname {
        detail.push_str(h);
    }
    if let Some(p) = e.port {
        if p != 22 {
            detail.push(':');
            detail.push_str(&p.to_string());
        }
    }
    if detail.is_empty() || detail == e.host || detail.trim_end_matches('@') == e.host {
        e.host.clone()
    } else {
        format!("{} -> {}", e.host, detail)
    }
}

pub fn run_picker(
    mode: PickerMode,
    store: &mut crate::db::store::Store,
    initial_query: Option<String>,
    open_config: bool,
) -> Result<Option<(PickerMode, String, bool)>> {
    app::run(mode, store, initial_query, open_config)
}

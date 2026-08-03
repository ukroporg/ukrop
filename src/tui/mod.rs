mod app;
pub mod config_dialog;
pub mod edit_dialog;
mod fuzzy;
mod input;
pub mod ranking;
pub mod theme;
mod tty_reader;
mod ui;

use anyhow::Result;

pub use app::{PickerMode, UnifiedList};

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
    pub cwd: Option<String>,
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
                    cwd: None,
                }
            })
            .collect()
    }

    pub fn from_cmds(entries: Vec<crate::db::model::CmdEntry>) -> Vec<Self> {
        entries
            .into_iter()
            .map(|e| {
                let duration_ms = e.duration_ms;
                let cwd = e.cwd.clone();
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
                    cwd,
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
                    cwd: None,
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

/// One entry in the unified list, tagged with which source it came from.
#[derive(Clone)]
pub struct Row {
    pub kind: PickerMode,
    pub entry: PickerEntry,
}

/// Which types the unified list is currently showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TypeFilter {
    All,
    Cd,
    Run,
    Ssh,
}

impl TypeFilter {
    pub fn next(self) -> Self {
        match self {
            TypeFilter::All => TypeFilter::Cd,
            TypeFilter::Cd => TypeFilter::Run,
            TypeFilter::Run => TypeFilter::Ssh,
            TypeFilter::Ssh => TypeFilter::All,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TypeFilter::All => TypeFilter::Ssh,
            TypeFilter::Cd => TypeFilter::All,
            TypeFilter::Run => TypeFilter::Cd,
            TypeFilter::Ssh => TypeFilter::Run,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TypeFilter::All => "All",
            TypeFilter::Cd => "cd",
            TypeFilter::Run => "run",
            TypeFilter::Ssh => "ssh",
        }
    }

    pub fn accepts(self, kind: PickerMode) -> bool {
        matches!(
            (self, kind),
            (TypeFilter::All, _)
                | (TypeFilter::Cd, PickerMode::Directories)
                | (TypeFilter::Run, PickerMode::Commands)
                | (TypeFilter::Ssh, PickerMode::SshHosts)
        )
    }
}

/// What the picker returns to `lib.rs` when the user selects something.
pub struct PickResult {
    pub kind: PickerMode,
    /// Text written to stdout after the `cd:`/`run:`/`ssh:` prefix.
    pub output: String,
    /// The row's database key, used to record a transition. Differs from `output`
    /// for SSH rows, where `output` is the full connection args.
    pub key: String,
    pub edit: bool,
}

pub fn run_picker(
    mode: Option<PickerMode>,
    store: &mut crate::db::store::Store,
    initial_query: Option<String>,
    open_config: bool,
) -> Result<Option<PickResult>> {
    app::run(mode, store, initial_query, open_config)
}

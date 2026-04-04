use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub id: i64,
    pub path: String,
    pub score: f64,
    pub visit_count: i64,
    pub last_visit: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdEntry {
    pub id: i64,
    pub command: String,
    pub score: f64,
    pub use_count: i64,
    pub last_used: i64,
    pub is_favorite: bool,
    pub source: String,
    pub exit_code: Option<i64>,
    pub cwd: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHostEntry {
    pub id: i64,
    pub host: String,
    pub hostname: Option<String>,
    pub port: Option<i32>,
    pub user: Option<String>,
    pub score: f64,
    pub use_count: i64,
    pub last_used: i64,
    pub is_favorite: bool,
    pub source: String,
}

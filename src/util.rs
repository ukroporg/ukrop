use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .context("Cannot determine data directory")?;
    let dir = base.join("ukrop");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn db_path() -> Result<String> {
    if let Ok(p) = std::env::var("UKROP_DB_PATH") {
        return Ok(p);
    }
    let path = data_dir()?.join("ukrop.db");
    Ok(path.to_string_lossy().into_owned())
}

pub fn resolve_path(path: &str) -> Result<String> {
    let expanded = if path.starts_with('~') {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        home.join(path.strip_prefix("~/").unwrap_or(&path[1..]))
    } else {
        PathBuf::from(path)
    };

    let canonical = std::fs::canonicalize(&expanded)
        .unwrap_or(expanded);

    Ok(canonical.to_string_lossy().into_owned())
}

pub fn short_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path.starts_with(home_str.as_ref()) {
            return format!("~{}", &path[home_str.len()..]);
        }
    }
    path.to_string()
}

use anyhow::Result;
use rusqlite::Connection;

use super::model::{CmdEntry, DirEntry, SshHostEntry};
use crate::frecency;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        super::migrate::run(&conn)?;
        Ok(Store { conn })
    }

    pub fn record_visit(&mut self, path: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        let now = chrono::Utc::now().timestamp();

        let existing = tx.query_row(
            "SELECT score, last_visit FROM directories WHERE path = ?1",
            [path],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)),
        );

        let new_score = match existing {
            Ok((old_score, last_visit)) => {
                let decayed = frecency::decay(old_score, last_visit, now);
                decayed + 1.0
            }
            Err(_) => 1.0,
        };

        tx.execute(
            "INSERT INTO directories (path, score, visit_count, last_visit)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(path) DO UPDATE SET
                score = ?2,
                visit_count = visit_count + 1,
                last_visit = ?3",
            rusqlite::params![path, new_score, now],
        )?;

        Self::age_directories_in(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_command(&mut self, command: &str, source: &str) -> Result<()> {
        self.record_command_full(command, source, None, None, None)
    }

    /// Import a command with a past timestamp so it doesn't appear recent.
    pub fn import_command(&mut self, command: &str, source: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600; // 30 days ago
        let tx = self.conn.transaction()?;

        let exists: bool = tx.query_row(
            "SELECT COUNT(*) > 0 FROM commands WHERE command = ?1",
            [command],
            |row| row.get(0),
        )?;

        if !exists {
            tx.execute(
                "INSERT INTO commands (command, score, use_count, last_used, source)
                 VALUES (?1, 1.0, 1, ?2, ?3)",
                rusqlite::params![command, past, source],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Import a directory visit with a past timestamp so it doesn't appear recent.
    pub fn import_visit(&mut self, path: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600; // 30 days ago
        let tx = self.conn.transaction()?;

        let exists: bool = tx.query_row(
            "SELECT COUNT(*) > 0 FROM directories WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;

        if !exists {
            tx.execute(
                "INSERT INTO directories (path, score, visit_count, last_visit)
                 VALUES (?1, 1.0, 1, ?2)",
                rusqlite::params![path, past],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Import an SSH host with a past timestamp so it doesn't appear recent.
    pub fn import_ssh_host(
        &mut self,
        host: &str,
        hostname: Option<&str>,
        port: Option<i32>,
        user: Option<&str>,
        source: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600; // 30 days ago
        let tx = self.conn.transaction()?;

        let exists: bool = tx.query_row(
            "SELECT COUNT(*) > 0 FROM ssh_hosts WHERE host = ?1",
            [host],
            |row| row.get(0),
        )?;

        if !exists {
            tx.execute(
                "INSERT INTO ssh_hosts (host, hostname, port, user, score, use_count, last_used, source)
                 VALUES (?1, ?2, ?3, ?4, 1.0, 1, ?5, ?6)",
                rusqlite::params![host, hostname, port, user, past, source],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn record_command_full(
        &mut self,
        command: &str,
        source: &str,
        exit_code: Option<i64>,
        cwd: Option<&str>,
        duration_ms: Option<i64>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let now = chrono::Utc::now().timestamp();

        let existing = tx.query_row(
            "SELECT score, last_used FROM commands WHERE command = ?1",
            [command],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)),
        );

        let new_score = match existing {
            Ok((old_score, last_used)) => {
                let decayed = frecency::decay(old_score, last_used, now);
                decayed + 1.0
            }
            Err(_) => 1.0,
        };

        tx.execute(
            "INSERT INTO commands (command, score, use_count, last_used, source, exit_code, cwd, duration_ms)
             VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(command) DO UPDATE SET
                score = ?2,
                use_count = use_count + 1,
                last_used = ?3,
                exit_code = ?5,
                cwd = COALESCE(?6, cwd),
                duration_ms = COALESCE(?7, duration_ms)",
            rusqlite::params![command, new_score, now, source, exit_code, cwd, duration_ms],
        )?;

        Self::age_commands_in(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_favorite(&mut self, path: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO directories (path, score, visit_count, last_visit, is_favorite)
             VALUES (?1, 1.0, 0, ?2, 1)
             ON CONFLICT(path) DO UPDATE SET is_favorite = 1",
            rusqlite::params![path, now],
        )?;
        Ok(())
    }

    pub fn toggle_favorite_dir(&mut self, path: &str) -> Result<bool> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE directories SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END WHERE path = ?1",
            [path],
        )?;
        let is_fav: bool = tx.query_row(
            "SELECT is_favorite FROM directories WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(is_fav)
    }

    pub fn toggle_favorite_cmd(&mut self, command: &str) -> Result<bool> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE commands SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END WHERE command = ?1",
            [command],
        )?;
        let is_fav: bool = tx.query_row(
            "SELECT is_favorite FROM commands WHERE command = ?1",
            [command],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(is_fav)
    }

    pub fn record_ssh_host(
        &mut self,
        host: &str,
        hostname: Option<&str>,
        port: Option<i32>,
        user: Option<&str>,
        source: &str,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let now = chrono::Utc::now().timestamp();

        let existing = tx.query_row(
            "SELECT score, last_used FROM ssh_hosts WHERE host = ?1",
            [host],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)),
        );

        let new_score = match existing {
            Ok((old_score, last_used)) => {
                let decayed = frecency::decay(old_score, last_used, now);
                decayed + 1.0
            }
            Err(_) => 1.0,
        };

        tx.execute(
            "INSERT INTO ssh_hosts (host, hostname, port, user, score, use_count, last_used, source)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)
             ON CONFLICT(host) DO UPDATE SET
                score = ?5,
                use_count = use_count + 1,
                last_used = ?6,
                hostname = COALESCE(?2, hostname),
                port = COALESCE(?3, port),
                user = COALESCE(?4, user)",
            rusqlite::params![host, hostname, port, user, new_score, now, source],
        )?;

        Self::age_ssh_hosts_in(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_ssh_hosts(&mut self) -> Result<Vec<SshHostEntry>> {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = self.conn.prepare(
            "SELECT id, host, hostname, port, user, score, use_count, last_used, is_favorite, source
             FROM ssh_hosts ORDER BY is_favorite DESC, score DESC",
        )?;
        let entries = stmt
            .query_map([], |row| {
                let score: f64 = row.get(5)?;
                let last_used: i64 = row.get(7)?;
                let decayed = frecency::decay(score, last_used, now);
                Ok(SshHostEntry {
                    id: row.get(0)?,
                    host: row.get(1)?,
                    hostname: row.get(2)?,
                    port: row.get(3)?,
                    user: row.get(4)?,
                    score: decayed,
                    use_count: row.get(6)?,
                    last_used,
                    is_favorite: row.get(8)?,
                    source: row.get(9)?,
                })
            })?
            .filter_map(|e| e.ok())
            .filter(|e| e.score >= 0.01 || e.is_favorite)
            .collect();
        Ok(entries)
    }

    pub fn toggle_favorite_ssh(&mut self, host: &str) -> Result<bool> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE ssh_hosts SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END WHERE host = ?1",
            [host],
        )?;
        let is_fav: bool = tx.query_row(
            "SELECT is_favorite FROM ssh_hosts WHERE host = ?1",
            [host],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(is_fav)
    }

    /// Try to match SSH command args against existing hosts and bump frecency.
    /// Matches by: exact host alias, hostname, user@hostname, or user@host.
    /// Returns true if a match was found.
    pub fn record_ssh_from_command(&mut self, args: &str) -> Result<bool> {
        // Parse the ssh target from args (skip flags like -p, -i, etc.)
        let target = parse_ssh_target(args);
        if target.is_empty() {
            return Ok(false);
        }

        // Split user@host if present
        let (cmd_user, cmd_host) = if let Some(pos) = target.find('@') {
            (Some(&target[..pos]), &target[pos + 1..])
        } else {
            (None, target.as_str())
        };

        // Try exact host alias match first
        let alias_match: Option<String> = self
            .conn
            .query_row(
                "SELECT host FROM ssh_hosts WHERE host = ?1",
                [cmd_host],
                |row| row.get(0),
            )
            .ok();

        if let Some(host) = alias_match {
            self.record_ssh_host(&host, None, None, None, "hook")?;
            return Ok(true);
        }

        // Try matching by hostname (with optional user)
        let matched_host: Option<String> = if let Some(user) = cmd_user {
            self.conn
                .query_row(
                    "SELECT host FROM ssh_hosts WHERE hostname = ?1 AND user = ?2",
                    rusqlite::params![cmd_host, user],
                    |row| row.get(0),
                )
                .ok()
                .or_else(|| {
                    self.conn
                        .query_row(
                            "SELECT host FROM ssh_hosts WHERE hostname = ?1",
                            [cmd_host],
                            |row| row.get(0),
                        )
                        .ok()
                })
        } else {
            self.conn
                .query_row(
                    "SELECT host FROM ssh_hosts WHERE hostname = ?1",
                    [cmd_host],
                    |row| row.get(0),
                )
                .ok()
        };

        if let Some(host) = matched_host {
            self.record_ssh_host(&host, None, None, None, "hook")?;
            return Ok(true);
        }

        // No existing match — record as new host
        self.record_ssh_host(&target, None, None, None, "hook")?;
        Ok(true)
    }

    pub fn forget(&mut self, target: &str) -> Result<bool> {
        let dir_deleted = self
            .conn
            .execute("DELETE FROM directories WHERE path = ?1", [target])?;
        let cmd_deleted = self
            .conn
            .execute("DELETE FROM commands WHERE command = ?1", [target])?;
        let ssh_deleted = self
            .conn
            .execute("DELETE FROM ssh_hosts WHERE host = ?1", [target])?;
        Ok(dir_deleted + cmd_deleted + ssh_deleted > 0)
    }

    pub fn list_directories(&mut self) -> Result<Vec<DirEntry>> {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = self.conn.prepare(
            "SELECT id, path, score, visit_count, last_visit, is_favorite FROM directories ORDER BY is_favorite DESC, score DESC",
        )?;
        let entries = stmt
            .query_map([], |row| {
                let score: f64 = row.get(2)?;
                let last_visit: i64 = row.get(4)?;
                let decayed = frecency::decay(score, last_visit, now);
                Ok(DirEntry {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    score: decayed,
                    visit_count: row.get(3)?,
                    last_visit,
                    is_favorite: row.get(5)?,
                })
            })?
            .filter_map(|e| e.ok())
            .filter(|e| e.score >= 0.01 || e.is_favorite)
            .collect();
        Ok(entries)
    }

    pub fn list_commands(&mut self) -> Result<Vec<CmdEntry>> {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = self.conn.prepare(
            "SELECT id, command, score, use_count, last_used, is_favorite, source, exit_code, cwd, duration_ms FROM commands ORDER BY is_favorite DESC, score DESC",
        )?;
        let entries = stmt
            .query_map([], |row| {
                let score: f64 = row.get(2)?;
                let last_used: i64 = row.get(4)?;
                let decayed = frecency::decay(score, last_used, now);
                Ok(CmdEntry {
                    id: row.get(0)?,
                    command: row.get(1)?,
                    score: decayed,
                    use_count: row.get(3)?,
                    last_used,
                    is_favorite: row.get(5)?,
                    source: row.get(6)?,
                    exit_code: row.get(7)?,
                    cwd: row.get(8)?,
                    duration_ms: row.get(9)?,
                })
            })?
            .filter_map(|e| e.ok())
            .filter(|e| e.score >= 0.01 || e.is_favorite)
            .collect();
        Ok(entries)
    }

    pub fn list_commands_by_cwd(&mut self, cwd: &str) -> Result<Vec<CmdEntry>> {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = self.conn.prepare(
            "SELECT id, command, score, use_count, last_used, is_favorite, source, exit_code, cwd, duration_ms FROM commands WHERE cwd = ?1 ORDER BY is_favorite DESC, score DESC",
        )?;
        let entries = stmt
            .query_map([cwd], |row| {
                let score: f64 = row.get(2)?;
                let last_used: i64 = row.get(4)?;
                let decayed = frecency::decay(score, last_used, now);
                Ok(CmdEntry {
                    id: row.get(0)?,
                    command: row.get(1)?,
                    score: decayed,
                    use_count: row.get(3)?,
                    last_used,
                    is_favorite: row.get(5)?,
                    source: row.get(6)?,
                    exit_code: row.get(7)?,
                    cwd: row.get(8)?,
                    duration_ms: row.get(9)?,
                })
            })?
            .filter_map(|e| e.ok())
            .filter(|e| e.score >= 0.01 || e.is_favorite)
            .collect();
        Ok(entries)
    }

    pub fn is_empty(&self) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT (SELECT COUNT(*) FROM directories) + (SELECT COUNT(*) FROM commands) + (SELECT COUNT(*) FROM ssh_hosts)",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Batch import commands with guessed cwd in a single transaction.
    /// Only sets cwd if the command doesn't already have one (preserves hook-recorded cwd).
    pub fn import_commands_with_cwd_batch(
        &mut self,
        commands: &[(String, Option<String>)],
        source: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600;
        let tx = self.conn.transaction()?;
        for (command, cwd) in commands {
            let exists: bool = tx.query_row(
                "SELECT COUNT(*) > 0 FROM commands WHERE command = ?1",
                [command.as_str()],
                |row| row.get(0),
            )?;
            if !exists {
                tx.execute(
                    "INSERT INTO commands (command, score, use_count, last_used, source, cwd)
                     VALUES (?1, 1.0, 1, ?2, ?3, ?4)",
                    rusqlite::params![command, past, source, cwd],
                )?;
            } else if cwd.is_some() {
                // Update cwd only if not already set
                tx.execute(
                    "UPDATE commands SET cwd = COALESCE(cwd, ?2) WHERE command = ?1",
                    rusqlite::params![command, cwd],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Batch import commands in a single transaction for performance.
    pub fn import_commands_batch(&mut self, commands: &[String], source: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600;
        let tx = self.conn.transaction()?;
        for command in commands {
            let exists: bool = tx.query_row(
                "SELECT COUNT(*) > 0 FROM commands WHERE command = ?1",
                [command.as_str()],
                |row| row.get(0),
            )?;
            if !exists {
                tx.execute(
                    "INSERT INTO commands (command, score, use_count, last_used, source)
                     VALUES (?1, 1.0, 1, ?2, ?3)",
                    rusqlite::params![command, past, source],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Batch import directory visits in a single transaction for performance.
    pub fn import_visits_batch(&mut self, paths: &[String]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600;
        let tx = self.conn.transaction()?;
        for path in paths {
            let exists: bool = tx.query_row(
                "SELECT COUNT(*) > 0 FROM directories WHERE path = ?1",
                [path.as_str()],
                |row| row.get(0),
            )?;
            if !exists {
                tx.execute(
                    "INSERT INTO directories (path, score, visit_count, last_visit)
                     VALUES (?1, 1.0, 1, ?2)",
                    rusqlite::params![path, past],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Batch import SSH hosts in a single transaction for performance.
    pub fn import_ssh_hosts_batch(
        &mut self,
        hosts: &[crate::ssh::config::SshConfigHost],
        source: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let past = now - 30 * 24 * 3600;
        let tx = self.conn.transaction()?;
        for h in hosts {
            let exists: bool = tx.query_row(
                "SELECT COUNT(*) > 0 FROM ssh_hosts WHERE host = ?1",
                [h.host.as_str()],
                |row| row.get(0),
            )?;
            if !exists {
                tx.execute(
                    "INSERT INTO ssh_hosts (host, hostname, port, user, score, use_count, last_used, source)
                     VALUES (?1, ?2, ?3, ?4, 1.0, 1, ?5, ?6)",
                    rusqlite::params![h.host, h.hostname, h.port, h.user, past, source],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove stale directory entries that no longer exist on disk.
    pub fn cleanup_stale_directories(&mut self, max_age_days: u64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let threshold_secs = (max_age_days * 24 * 3600) as i64;
        let mut stmt = self.conn.prepare(
            "SELECT path, last_visit, score FROM directories WHERE is_favorite = 0",
        )?;
        let stale: Vec<String> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter(|(path, last_visit, score)| {
                let age = now - last_visit;
                age > threshold_secs
                    && *score < 1.0
                    && !std::path::Path::new(path).is_dir()
            })
            .map(|(path, _, _)| path)
            .collect();
        let count = stale.len();
        for path in &stale {
            self.conn.execute("DELETE FROM directories WHERE path = ?1", [path])?;
        }
        Ok(count)
    }

    /// Best match directory for non-interactive mode.
    pub fn best_match_directory(&mut self, query: &str) -> Result<Option<String>> {
        let now = chrono::Utc::now().timestamp();
        let entries = self.list_directories()?;
        let query_lower = query.to_lowercase();

        // Score each entry: exact basename match > prefix > contains > fuzzy
        let mut best: Option<(String, f64)> = None;
        for e in &entries {
            let path_lower = e.path.to_lowercase();
            let basename = std::path::Path::new(&e.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            let match_score = if basename == query_lower {
                1000.0
            } else if basename.starts_with(&query_lower) {
                500.0
            } else if path_lower.contains(&query_lower) {
                100.0
            } else {
                continue;
            };

            let decayed = frecency::decay(e.score, e.last_visit, now);
            let total = match_score + decayed;

            if best.as_ref().map(|(_, s)| total > *s).unwrap_or(true) {
                best = Some((e.path.clone(), total));
            }
        }
        Ok(best.map(|(p, _)| p))
    }

    fn age_table_in(conn: &rusqlite::Connection, table: &str) -> Result<()> {
        let total: f64 = conn
            .query_row(
                &format!("SELECT COALESCE(SUM(score), 0.0) FROM {}", table),
                [],
                |row| row.get(0),
            )?;
        if total > frecency::AGE_THRESHOLD {
            let scale = (frecency::AGE_THRESHOLD * 0.9) / total;
            conn.execute(
                &format!("UPDATE {} SET score = score * ?1", table),
                [scale],
            )?;
            conn.execute(
                &format!("DELETE FROM {} WHERE score < 0.01 AND is_favorite = 0", table),
                [],
            )?;
        }
        Ok(())
    }

    fn age_directories_in(conn: &rusqlite::Connection) -> Result<()> {
        Self::age_table_in(conn, "directories")
    }

    fn age_commands_in(conn: &rusqlite::Connection) -> Result<()> {
        Self::age_table_in(conn, "commands")
    }

    fn age_ssh_hosts_in(conn: &rusqlite::Connection) -> Result<()> {
        Self::age_table_in(conn, "ssh_hosts")
    }
}

/// Extract the SSH target (user@host or host) from ssh command args,
/// skipping flags like -p, -i, -o, etc.
fn parse_ssh_target(args: &str) -> String {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if part.starts_with('-') {
            // Flags that take a value: skip the next arg too
            match part {
                "-p" | "-i" | "-l" | "-o" | "-F" | "-J" | "-W" | "-b" | "-c" | "-D" | "-E"
                | "-e" | "-I" | "-L" | "-m" | "-O" | "-Q" | "-R" | "-S" | "-w" => {
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        } else {
            // First non-flag argument is the target
            return part.to_string();
        }
    }
    String::new()
}

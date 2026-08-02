use anyhow::Result;
use std::collections::HashMap;

use super::store::Store;
use crate::frecency;

impl Store {
    /// Record a jump from `from_cwd` to `target`. `kind` is "cd" or "ssh".
    /// Score accumulates with the same 1-week-half-life decay as directories and commands.
    pub fn record_transition(&mut self, from_cwd: &str, kind: &str, target: &str) -> Result<()> {
        let n = self.record_transitions_batch(&[(
            from_cwd.to_string(),
            kind.to_string(),
            target.to_string(),
        )])?;
        debug_assert_eq!(n, 1);
        Ok(())
    }

    /// Batch form of `record_transition`. Returns the number of items processed.
    pub fn record_transitions_batch(&mut self, items: &[(String, String, String)]) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let tx = self.conn_mut().transaction()?;
        for (from_cwd, kind, target) in items {
            let existing = tx.query_row(
                "SELECT score, last_time FROM transitions
                 WHERE from_cwd = ?1 AND kind = ?2 AND target = ?3",
                rusqlite::params![from_cwd, kind, target],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)),
            );
            let new_score = match existing {
                Ok((old, last_time)) => frecency::decay(old, last_time, now) + 1.0,
                Err(_) => 1.0,
            };
            tx.execute(
                "INSERT INTO transitions (from_cwd, kind, target, score, count, last_time)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5)
                 ON CONFLICT(from_cwd, kind, target) DO UPDATE SET
                    score = ?4,
                    count = count + 1,
                    last_time = ?5",
                rusqlite::params![from_cwd, kind, target, new_score, now],
            )?;
        }
        tx.commit()?;
        Ok(items.len())
    }

    /// All transitions originating at `from_cwd`, keyed by (kind, target), with decayed scores.
    pub fn transitions_from(&self, from_cwd: &str) -> Result<HashMap<(String, String), f64>> {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = self.conn_ref().prepare(
            "SELECT kind, target, score, last_time FROM transitions WHERE from_cwd = ?1",
        )?;
        let rows = stmt.query_map([from_cwd], |row| {
            let kind: String = row.get(0)?;
            let target: String = row.get(1)?;
            let score: f64 = row.get(2)?;
            let last_time: i64 = row.get(3)?;
            Ok(((kind, target), frecency::decay(score, last_time, now)))
        })?;
        Ok(rows.filter_map(|r| r.ok()).filter(|(_, s)| *s >= 0.01).collect())
    }

    /// Delete transitions untouched for longer than `max_age_days`.
    pub fn prune_transitions(&mut self, max_age_days: u64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (max_age_days * 24 * 3600) as i64;
        let n = self
            .conn_mut()
            .execute("DELETE FROM transitions WHERE last_time < ?1", [cutoff])?;
        Ok(n)
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let v = self
            .conn_ref()
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .ok();
        Ok(v)
    }

    pub fn set_meta(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn_mut().execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// Last recorded PWD for a shell instance, or None if never recorded.
    pub fn get_shell_pwd(&self, shell_id: &str) -> Result<Option<String>> {
        let raw = match self.get_meta(&format!("last_pwd:{}", shell_id))? {
            Some(v) => v,
            None => return Ok(None),
        };
        // Stored as "<timestamp>\t<path>"; tolerate a bare path from any older write.
        Ok(match raw.split_once('\t') {
            Some((_, path)) => Some(path.to_string()),
            None => Some(raw),
        })
    }

    pub fn set_shell_pwd(&mut self, shell_id: &str, pwd: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.set_meta(&format!("last_pwd:{}", shell_id), &format!("{}\t{}", now, pwd))
    }

    /// Delete `last_pwd:*` meta keys whose timestamp is older than `max_age_days`.
    pub fn prune_shell_pwd_keys(&mut self, max_age_days: u64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (max_age_days * 24 * 3600) as i64;
        let stale: Vec<String> = {
            let mut stmt = self
                .conn_ref()
                .prepare("SELECT key, value FROM meta WHERE key LIKE 'last_pwd:%'")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.filter_map(|r| r.ok())
                .filter(|(_, v)| {
                    v.split_once('\t')
                        .and_then(|(ts, _)| ts.parse::<i64>().ok())
                        .map(|ts| ts < cutoff)
                        .unwrap_or(true) // malformed / legacy value: prune it
                })
                .map(|(k, _)| k)
                .collect()
        };
        let mut n = 0;
        for key in &stale {
            n += self
                .conn_mut()
                .execute("DELETE FROM meta WHERE key = ?1", [key])?;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::store::Store;

    fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let s = Store::open(path.to_str().unwrap()).unwrap();
        (s, dir)
    }

    #[test]
    fn test_record_and_read_transition() {
        let (mut s, _d) = store();
        s.record_transition("/home/me/proj", "cd", "/home/me/proj/src").unwrap();
        let map = s.transitions_from("/home/me/proj").unwrap();
        let v = map.get(&("cd".to_string(), "/home/me/proj/src".to_string()));
        assert!(v.is_some(), "transition should be readable back");
        assert!((v.unwrap() - 1.0).abs() < 0.01, "first record scores 1.0");
    }

    #[test]
    fn test_repeated_records_accumulate() {
        let (mut s, _d) = store();
        for _ in 0..3 {
            s.record_transition("/a", "ssh", "prod").unwrap();
        }
        let map = s.transitions_from("/a").unwrap();
        let v = *map.get(&("ssh".to_string(), "prod".to_string())).unwrap();
        assert!(v > 2.9 && v < 3.1, "three records with no elapsed time ~= 3.0, got {}", v);
    }

    #[test]
    fn test_transitions_are_scoped_to_from_cwd() {
        let (mut s, _d) = store();
        s.record_transition("/a", "cd", "/x").unwrap();
        s.record_transition("/b", "cd", "/y").unwrap();
        let map = s.transitions_from("/a").unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&("cd".to_string(), "/x".to_string())));
    }

    #[test]
    fn test_unknown_cwd_returns_empty() {
        let (s, _d) = store();
        assert!(s.transitions_from("/nowhere").unwrap().is_empty());
    }

    #[test]
    fn test_batch_insert() {
        let (mut s, _d) = store();
        let items = vec![
            ("/a".to_string(), "cd".to_string(), "/x".to_string()),
            ("/a".to_string(), "cd".to_string(), "/x".to_string()),
            ("/a".to_string(), "ssh".to_string(), "h1".to_string()),
        ];
        let n = s.record_transitions_batch(&items).unwrap();
        assert_eq!(n, 3, "batch reports rows processed");
        let map = s.transitions_from("/a").unwrap();
        assert_eq!(map.len(), 2, "duplicate (from,kind,target) merges into one row");
        let v = *map.get(&("cd".to_string(), "/x".to_string())).unwrap();
        assert!(v > 1.9, "duplicate accumulated, got {}", v);
    }

    #[test]
    fn test_shell_pwd_roundtrip() {
        let (mut s, _d) = store();
        assert_eq!(s.get_shell_pwd("123").unwrap(), None);
        s.set_shell_pwd("123", "/home/me").unwrap();
        assert_eq!(s.get_shell_pwd("123").unwrap(), Some("/home/me".to_string()));
        s.set_shell_pwd("123", "/tmp").unwrap();
        assert_eq!(s.get_shell_pwd("123").unwrap(), Some("/tmp".to_string()));
    }

    #[test]
    fn test_shell_pwd_is_per_id() {
        let (mut s, _d) = store();
        s.set_shell_pwd("1", "/a").unwrap();
        s.set_shell_pwd("2", "/b").unwrap();
        assert_eq!(s.get_shell_pwd("1").unwrap(), Some("/a".to_string()));
        assert_eq!(s.get_shell_pwd("2").unwrap(), Some("/b".to_string()));
    }

    #[test]
    fn test_prune_leaves_fresh_rows() {
        let (mut s, _d) = store();
        s.record_transition("/a", "cd", "/x").unwrap();
        s.set_shell_pwd("1", "/a").unwrap();
        assert_eq!(s.prune_transitions(90).unwrap(), 0);
        assert_eq!(s.prune_shell_pwd_keys(90).unwrap(), 0);
        assert_eq!(s.transitions_from("/a").unwrap().len(), 1);
        assert_eq!(s.get_shell_pwd("1").unwrap(), Some("/a".to_string()));
    }

    #[test]
    fn test_migration_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let p = path.to_str().unwrap();
        {
            let mut s = Store::open(p).unwrap();
            s.record_transition("/a", "cd", "/x").unwrap();
        }
        // Reopening runs migrate::run again; data must survive.
        let s = Store::open(p).unwrap();
        assert_eq!(s.transitions_from("/a").unwrap().len(), 1);
    }
}

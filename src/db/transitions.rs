use anyhow::Result;
use rusqlite::OptionalExtension;
use std::collections::HashMap;

use super::store::Store;
use crate::frecency;
use crate::tui::PickerMode;

/// Decayed transition scores from a single origin directory, split by kind so
/// lookups borrow instead of allocating a tuple key on every row.
///
/// A `HashMap<(String, String), f64>` cannot be probed with `(&str, &str)`
/// (`Borrow` is not implemented for tuple keys), which forced two `String`
/// allocations per row per keystroke on the ranking path.
#[derive(Debug, Clone, Default)]
pub struct Transitions {
    cd: HashMap<String, f64>,
    ssh: HashMap<String, f64>,
}

impl Transitions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decayed score for a jump to `target` of this `kind`, or 0.0 if none.
    /// Allocation-free: `HashMap<String, f64>::get` borrows the key.
    ///
    /// `PickerMode::Commands` always scores 0.0 — commands are tied to a cwd
    /// through `PickerEntry::cwd`, never through the transitions table.
    pub fn score(&self, kind: PickerMode, target: &str) -> f64 {
        let map = match kind {
            PickerMode::Directories => &self.cd,
            PickerMode::SshHosts => &self.ssh,
            PickerMode::Commands => return 0.0,
        };
        map.get(target).copied().unwrap_or(0.0)
    }

    /// Record a decayed score under the DB `kind` string ("cd" / "ssh").
    /// Any other kind is ignored — nothing else is ever written to the table,
    /// and `score` could not reach it anyway.
    pub fn insert(&mut self, kind: &str, target: String, score: f64) {
        match kind {
            "cd" => {
                self.cd.insert(target, score);
            }
            "ssh" => {
                self.ssh.insert(target, score);
            }
            _ => {}
        }
    }

    pub fn len(&self) -> usize {
        self.cd.len() + self.ssh.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cd.is_empty() && self.ssh.is_empty()
    }
}

/// Terse construction for tests: `Transitions::from([("cd", "/x", 20.0)])`.
impl<const N: usize> From<[(&str, &str, f64); N]> for Transitions {
    fn from(items: [(&str, &str, f64); N]) -> Self {
        let mut t = Transitions::new();
        for (kind, target, score) in items {
            t.insert(kind, target.to_string(), score);
        }
        t
    }
}

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
            // `.optional()` maps *only* QueryReturnedNoRows to None; every other
            // error (IO, corruption, a schema or type mismatch) propagates. A
            // blanket `Err(_) => 1.0` would silently reclassify those as "first
            // ever jump" and overwrite the accumulated score with 1.0.
            let existing = tx
                .query_row(
                    "SELECT score, last_time FROM transitions
                     WHERE from_cwd = ?1 AND kind = ?2 AND target = ?3",
                    rusqlite::params![from_cwd, kind, target],
                    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?;
            let new_score = match existing {
                Some((old, last_time)) => frecency::decay(old, last_time, now) + 1.0,
                None => 1.0,
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

    /// All transitions originating at `from_cwd`, with decayed scores.
    pub fn transitions_from(&self, from_cwd: &str) -> Result<Transitions> {
        let now = chrono::Utc::now().timestamp();
        let mut stmt = self.conn_ref().prepare(
            "SELECT kind, target, score, last_time FROM transitions WHERE from_cwd = ?1",
        )?;
        let rows = stmt.query_map([from_cwd], |row| {
            let kind: String = row.get(0)?;
            let target: String = row.get(1)?;
            let score: f64 = row.get(2)?;
            let last_time: i64 = row.get(3)?;
            Ok((kind, target, frecency::decay(score, last_time, now)))
        })?;
        let mut out = Transitions::new();
        for (kind, target, score) in rows.filter_map(|r| r.ok()) {
            if score >= 0.01 {
                out.insert(&kind, target, score);
            }
        }
        Ok(out)
    }

    /// Delete transitions untouched for longer than `max_age_days`.
    pub fn prune_transitions(&mut self, max_age_days: u64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (max_age_days * 24 * 3600) as i64;
        // Deliberately `<=`, not `<`: with `max_age_days == 0`, `cutoff == now()`,
        // and a row recorded in the same wall-clock second as this call has
        // `last_time == cutoff`. A zero-day window must still be able to prune
        // that row (see test_cleanup_prunes_stale_transitions_and_pwd_keys), so a
        // strict `<` would silently never match same-second rows. This is a
        // deliberate difference from `cleanup_stale_directories`'s strict `<`
        // (store.rs), which doesn't need to handle a zero-day boundary.
        let n = self
            .conn_mut()
            .execute("DELETE FROM transitions WHERE last_time <= ?1", [cutoff])?;
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
                        // Deliberately `<=`, not `<` — see the comment on
                        // prune_transitions above: a zero-day window must
                        // still prune a key written in the same second.
                        .map(|ts| ts <= cutoff)
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
    use crate::tui::PickerMode;

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
        let v = map.score(PickerMode::Directories, "/home/me/proj/src");
        assert!(v > 0.0, "transition should be readable back");
        assert!((v - 1.0).abs() < 0.01, "first record scores 1.0");
    }

    #[test]
    fn test_repeated_records_accumulate() {
        let (mut s, _d) = store();
        for _ in 0..3 {
            s.record_transition("/a", "ssh", "prod").unwrap();
        }
        let map = s.transitions_from("/a").unwrap();
        let v = map.score(PickerMode::SshHosts, "prod");
        assert!(v > 2.9 && v < 3.1, "three records with no elapsed time ~= 3.0, got {}", v);
    }

    #[test]
    fn test_transitions_are_scoped_to_from_cwd() {
        let (mut s, _d) = store();
        s.record_transition("/a", "cd", "/x").unwrap();
        s.record_transition("/b", "cd", "/y").unwrap();
        let map = s.transitions_from("/a").unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.score(PickerMode::Directories, "/x") > 0.0);
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
        let v = map.score(PickerMode::Directories, "/x");
        assert!(v > 1.9, "duplicate accumulated, got {}", v);
    }

    /// A SELECT failure that is *not* "no such row" must abort the write, not
    /// be silently treated as a first-ever jump.
    ///
    /// The fixture writes a non-numeric `score` straight into the table (SQLite
    /// has no strict typing, so a REAL column will hold TEXT), which makes
    /// `row.get::<_, f64>(0)` fail with `InvalidColumnType`. The follow-up
    /// INSERT/UPSERT is perfectly valid, so under the old `Err(_) => 1.0` this
    /// call returned `Ok` and clobbered the stored score with 1.0 — real
    /// corruption reported as success. Nothing here can produce
    /// QueryReturnedNoRows, so it pins error propagation specifically.
    #[test]
    fn test_unexpected_select_error_propagates() {
        let (mut s, _d) = store();
        s.conn_mut()
            .execute(
                "INSERT INTO transitions (from_cwd, kind, target, score, count, last_time)
                 VALUES ('/a', 'cd', '/x', 'not-a-number', 7, 0)",
                [],
            )
            .unwrap();

        let res = s.record_transition("/a", "cd", "/x");
        assert!(res.is_err(), "a corrupt row must surface as an error, not score 1.0");

        // And the bad row is left untouched: the transaction rolled back.
        let count: i64 = s
            .conn_ref()
            .query_row("SELECT count FROM transitions WHERE target = '/x'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 7, "the failed call must not have written anything");
    }

    /// The sibling of the test above: a genuinely absent row is still the
    /// 1.0 path, not an error.
    #[test]
    fn test_missing_row_is_not_an_error() {
        let (mut s, _d) = store();
        s.record_transition("/fresh", "cd", "/target").unwrap();
        let v = s.transitions_from("/fresh").unwrap().score(PickerMode::Directories, "/target");
        assert!((v - 1.0).abs() < 0.01, "absent row still scores 1.0, got {}", v);
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

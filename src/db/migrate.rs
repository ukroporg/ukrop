use anyhow::Result;
use rusqlite::Connection;

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stmt.query_map([], |row| row.get::<_, String>(1))
        .map(|rows| rows.filter_map(|r| r.ok()).any(|name| name == column))
        .unwrap_or(false)
}

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS directories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                score REAL NOT NULL DEFAULT 0.0,
                visit_count INTEGER NOT NULL DEFAULT 0,
                last_visit INTEGER NOT NULL DEFAULT 0,
                is_favorite INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS commands (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command TEXT NOT NULL UNIQUE,
                score REAL NOT NULL DEFAULT 0.0,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_used INTEGER NOT NULL DEFAULT 0,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'hook'
            );

            INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '1');",
        )?;
    }

    if version < 2 {
        // Check which columns already exist to handle partial migrations
        let has_exit_code = has_column(conn, "commands", "exit_code");
        let has_cwd = has_column(conn, "commands", "cwd");
        if !has_exit_code {
            conn.execute_batch("ALTER TABLE commands ADD COLUMN exit_code INTEGER;")?;
        }
        if !has_cwd {
            conn.execute_batch("ALTER TABLE commands ADD COLUMN cwd TEXT;")?;
        }
        conn.execute_batch(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '2');",
        )?;
    }

    if version < 3 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ssh_hosts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                host TEXT NOT NULL UNIQUE,
                hostname TEXT,
                port INTEGER,
                user TEXT,
                score REAL NOT NULL DEFAULT 0.0,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_used INTEGER NOT NULL DEFAULT 0,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'config'
            );

            INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '3');",
        )?;
    }

    if version < 4 {
        if !has_column(conn, "commands", "duration_ms") {
            conn.execute_batch("ALTER TABLE commands ADD COLUMN duration_ms INTEGER;")?;
        }
        conn.execute_batch(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '4');",
        )?;
    }

    if version < 5 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transitions (
                from_cwd  TEXT    NOT NULL,
                kind      TEXT    NOT NULL,
                target    TEXT    NOT NULL,
                score     REAL    NOT NULL DEFAULT 0.0,
                count     INTEGER NOT NULL DEFAULT 0,
                last_time INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (from_cwd, kind, target)
            );

            CREATE INDEX IF NOT EXISTS idx_transitions_from ON transitions(from_cwd);

            INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', '5');",
        )?;
    }

    Ok(())
}

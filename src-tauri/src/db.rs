use rusqlite::{Connection, Result};

const SCHEMA_V1: &str = "
CREATE TABLE apps (
  id            INTEGER PRIMARY KEY,
  identity      TEXT NOT NULL UNIQUE,
  display_name  TEXT NOT NULL,
  process_name  TEXT,
  exe_path      TEXT,
  bundle_id     TEXT,
  icon_path     TEXT,
  created_at    INTEGER NOT NULL
);

CREATE TABLE segments (
  id            INTEGER PRIMARY KEY,
  app_id        INTEGER NOT NULL REFERENCES apps(id),
  window_title  TEXT,
  state         TEXT NOT NULL CHECK (state IN ('active','idle')),
  start_ts      INTEGER NOT NULL,
  end_ts        INTEGER NOT NULL,
  duration_sec  INTEGER NOT NULL,
  day           TEXT NOT NULL
);
CREATE INDEX idx_segments_day ON segments(day);
CREATE INDEX idx_segments_app ON segments(app_id);

CREATE TABLE usage_daily (
  day        TEXT NOT NULL,
  app_id     INTEGER NOT NULL REFERENCES apps(id),
  active_sec INTEGER NOT NULL DEFAULT 0,
  idle_sec   INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day, app_id)
);

CREATE TABLE config (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";

const MIGRATIONS: &[&str] = &[SCHEMA_V1];

pub struct Db {
    pub(crate) conn: Connection,
}

impl Db {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Self::from_conn(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_conn(Connection::open_in_memory()?)
    }

    fn from_conn(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Db { conn };
        db.apply_migrations()?;
        Ok(db)
    }

    pub fn apply_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_meta (version INTEGER NOT NULL);",
        )?;
        let current = self.schema_version()?;
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let version = (i + 1) as i64;
            if version > current {
                self.conn.execute_batch(sql)?;
                self.conn.execute("DELETE FROM schema_meta", [])?;
                self.conn
                    .execute("INSERT INTO schema_meta (version) VALUES (?1)", [version])?;
            }
        }
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_meta",
            [],
            |r| r.get(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_names(db: &Db) -> Vec<String> {
        let mut stmt = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap()
    }

    #[test]
    fn open_in_memory_applies_schema_v1() {
        let db = Db::open_in_memory().unwrap();

        assert_eq!(db.schema_version().unwrap(), 1);

        let tables = table_names(&db);
        for t in ["apps", "segments", "usage_daily", "config", "schema_meta"] {
            assert!(tables.iter().any(|n| n == t), "missing table {t}");
        }
    }

    #[test]
    fn reopening_does_not_rerun_migrations() {
        let db = Db::open_in_memory().unwrap();
        db.apply_migrations().unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
    }
}

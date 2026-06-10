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

/// Identidad observada de una app en un tick. `identity` es la clave
/// canónica: bundle_id en macOS, exe_path en Windows.
#[derive(Debug, Clone)]
pub struct AppInfo<'a> {
    pub identity: &'a str,
    pub display_name: &'a str,
    pub process_name: Option<&'a str>,
    pub exe_path: Option<&'a str>,
    pub bundle_id: Option<&'a str>,
}

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

    /// Inserta la app si no existe (por `identity`) y devuelve su id.
    /// Refresca metadatos en upserts posteriores pero NUNCA pisa
    /// `display_name`: es editable por el usuario.
    pub fn upsert_app(&self, app: &AppInfo, now_ts: i64) -> Result<i64> {
        self.conn.query_row(
            "INSERT INTO apps (identity, display_name, process_name, exe_path, bundle_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(identity) DO UPDATE SET
               process_name = excluded.process_name,
               exe_path     = excluded.exe_path,
               bundle_id    = excluded.bundle_id
             RETURNING id",
            rusqlite::params![
                app.identity,
                app.display_name,
                app.process_name,
                app.exe_path,
                app.bundle_id,
                now_ts
            ],
            |r| r.get(0),
        )
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

    fn safari() -> AppInfo<'static> {
        AppInfo {
            identity: "com.apple.Safari",
            display_name: "Safari",
            process_name: Some("Safari"),
            exe_path: None,
            bundle_id: Some("com.apple.Safari"),
        }
    }

    #[test]
    fn upsert_app_is_idempotent_by_identity() {
        let db = Db::open_in_memory().unwrap();
        let id1 = db.upsert_app(&safari(), 1000).unwrap();
        let id2 = db.upsert_app(&safari(), 2000).unwrap();
        assert_eq!(id1, id2);

        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM apps", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn upsert_app_does_not_overwrite_display_name() {
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_app(&safari(), 1000).unwrap();

        let renamed = AppInfo {
            display_name: "Mi Safari",
            ..safari()
        };
        db.upsert_app(&renamed, 2000).unwrap();

        let name: String = db
            .conn
            .query_row("SELECT display_name FROM apps WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "Safari");
    }

    #[test]
    fn upsert_app_refreshes_metadata() {
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_app(&safari(), 1000).unwrap();

        let moved = AppInfo {
            exe_path: Some("/Applications/Safari.app"),
            ..safari()
        };
        db.upsert_app(&moved, 2000).unwrap();

        let path: Option<String> = db
            .conn
            .query_row("SELECT exe_path FROM apps WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(path.as_deref(), Some("/Applications/Safari.app"));
    }
}

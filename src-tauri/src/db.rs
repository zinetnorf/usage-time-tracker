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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegState {
    Active,
    Idle,
}

impl SegState {
    fn as_str(self) -> &'static str {
        match self {
            SegState::Active => "active",
            SegState::Idle => "idle",
        }
    }
}

/// Día local 'YYYY-MM-DD' para un epoch en segundos (§6: frontera del día
/// en hora local).
pub fn local_day(ts: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(ts, 0)
        .single()
        .expect("timestamp válido")
        .format("%Y-%m-%d")
        .to_string()
}

/// Epoch de la medianoche local siguiente a `ts` (maneja DST vía chrono).
fn next_local_midnight(ts: i64) -> i64 {
    use chrono::{Duration, Local, TimeZone};
    let dt = Local.timestamp_opt(ts, 0).single().expect("timestamp válido");
    let next_day = dt.date_naive() + Duration::days(1);
    Local
        .from_local_datetime(&next_day.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .expect("medianoche local válida")
        .timestamp()
}

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

    /// Abre un segmento con `end_ts` provisional = `start_ts`.
    pub fn open_segment(
        &self,
        app_id: i64,
        window_title: Option<&str>,
        state: SegState,
        start_ts: i64,
    ) -> Result<i64> {
        let id: i64 = self.conn.query_row(
            "INSERT INTO segments (app_id, window_title, state, start_ts, end_ts, duration_sec, day)
             VALUES (?1, ?2, ?3, ?4, ?4, 0, ?5)
             RETURNING id",
            rusqlite::params![app_id, window_title, state.as_str(), start_ts, local_day(start_ts)],
            |r| r.get(0),
        )?;
        // Marker para recovery (§13): identifica el único segmento abierto.
        self.set_config("open_segment_id", &id.to_string())?;
        Ok(id)
    }

    /// Persiste avance del segmento abierto (anti-crash, §5.2.5).
    /// No toca el rollup: eso ocurre solo al cerrar.
    pub fn flush_segment(&self, segment_id: i64, end_ts: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE segments
             SET end_ts = ?2, duration_sec = ?2 - start_ts
             WHERE id = ?1",
            rusqlite::params![segment_id, end_ts],
        )?;
        Ok(())
    }

    /// Cierra el segmento y acumula su duración en `usage_daily`.
    /// Si cruza medianoche local se parte en un segmento por día (§6).
    pub fn close_segment(&self, segment_id: i64, end_ts: i64) -> Result<()> {
        let mut seg_id = segment_id;
        loop {
            let start_ts: i64 = self.conn.query_row(
                "SELECT start_ts FROM segments WHERE id = ?1",
                [seg_id],
                |r| r.get(0),
            )?;
            let boundary = next_local_midnight(start_ts);
            if end_ts <= boundary {
                self.close_segment_piece(seg_id, end_ts)?;
                self.set_config("open_segment_id", "")?;
                return Ok(());
            }

            self.close_segment_piece(seg_id, boundary)?;
            seg_id = self.conn.query_row(
                "INSERT INTO segments (app_id, window_title, state, start_ts, end_ts, duration_sec, day)
                 SELECT app_id, window_title, state, ?2, ?2, 0, ?3
                 FROM segments WHERE id = ?1
                 RETURNING id",
                rusqlite::params![seg_id, boundary, local_day(boundary)],
                |r| r.get(0),
            )?;
        }
    }

    fn close_segment_piece(&self, segment_id: i64, end_ts: i64) -> Result<()> {
        self.flush_segment(segment_id, end_ts)?;
        self.conn.execute(
            "INSERT INTO usage_daily (day, app_id, active_sec, idle_sec)
             SELECT day, app_id,
                    CASE state WHEN 'active' THEN duration_sec ELSE 0 END,
                    CASE state WHEN 'idle'   THEN duration_sec ELSE 0 END
             FROM segments WHERE id = ?1
             ON CONFLICT(day, app_id) DO UPDATE SET
               active_sec = active_sec + excluded.active_sec,
               idle_sec   = idle_sec   + excluded.idle_sec",
            [segment_id],
        )?;
        Ok(())
    }

    pub fn rename_app(&self, app_id: i64, display_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE apps SET display_name = ?2 WHERE id = ?1",
            rusqlite::params![app_id, display_name],
        )?;
        Ok(())
    }

    /// Fusiona `from` dentro de `into`: reasigna segmentos, suma rollups
    /// y elimina la app origen. Atómico.
    pub fn merge_apps(&self, from: i64, into: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE segments SET app_id = ?2 WHERE app_id = ?1",
            [from, into],
        )?;
        tx.execute(
            "INSERT INTO usage_daily (day, app_id, active_sec, idle_sec)
             SELECT day, ?2, active_sec, idle_sec FROM usage_daily WHERE app_id = ?1
             ON CONFLICT(day, app_id) DO UPDATE SET
               active_sec = active_sec + excluded.active_sec,
               idle_sec   = idle_sec   + excluded.idle_sec",
            [from, into],
        )?;
        tx.execute("DELETE FROM usage_daily WHERE app_id = ?1", [from])?;
        tx.execute("DELETE FROM apps WHERE id = ?1", [from])?;
        tx.commit()
    }

    /// Recovery de arranque (§13): si quedó un segmento abierto de una
    /// sesión previa, lo cierra en su último `end_ts` flusheado. No
    /// rellena el hueco hasta ahora.
    pub fn recover(&self) -> Result<Option<i64>> {
        let marker = self.config_str("open_segment_id")?;
        let Ok(seg_id) = marker.parse::<i64>() else {
            return Ok(None);
        };
        let end_ts: i64 = self.conn.query_row(
            "SELECT end_ts FROM segments WHERE id = ?1",
            [seg_id],
            |r| r.get(0),
        )?;
        self.close_segment(seg_id, end_ts)?;
        Ok(Some(seg_id))
    }

    /// Defaults del spec §7. Clave desconocida → cadena vacía.
    fn config_default(key: &str) -> &'static str {
        match key {
            "idle_threshold_sec" => "60",
            "count_idle_as_usage" => "true",
            "track_window_titles" => "true",
            "poll_interval_ms" => "1500",
            "flush_interval_sec" => "30",
            "raw_retention_days" => "180",
            "autostart_enabled" => "true",
            "language" => "es",
            "top_apps_count" => "10",
            "tracking_paused" => "false",
            _ => "",
        }
    }

    pub fn config_str(&self, key: &str) -> Result<String> {
        let stored: Option<String> = self
            .conn
            .query_row("SELECT value FROM config WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(stored.unwrap_or_else(|| Self::config_default(key).to_string()))
    }

    pub fn config_i64(&self, key: &str) -> Result<i64> {
        Ok(self.config_str(key)?.parse().unwrap_or_else(|_| {
            Self::config_default(key).parse().unwrap_or(0)
        }))
    }

    pub fn config_bool(&self, key: &str) -> Result<bool> {
        Ok(self.config_str(key)? == "true")
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
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

    fn rollup(db: &Db, day: &str, app_id: i64) -> (i64, i64) {
        db.conn
            .query_row(
                "SELECT active_sec, idle_sec FROM usage_daily WHERE day = ?1 AND app_id = ?2",
                rusqlite::params![day, app_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap()
    }

    #[test]
    fn open_segment_starts_with_provisional_end() {
        let db = Db::open_in_memory().unwrap();
        let app = db.upsert_app(&safari(), 1000).unwrap();
        let seg = db
            .open_segment(app, Some("Inicio"), SegState::Active, 1000)
            .unwrap();

        let (end_ts, dur, day): (i64, i64, String) = db
            .conn
            .query_row(
                "SELECT end_ts, duration_sec, day FROM segments WHERE id = ?1",
                [seg],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(end_ts, 1000);
        assert_eq!(dur, 0);
        assert_eq!(day, local_day(1000));
    }

    #[test]
    fn flush_segment_updates_end_without_rollup() {
        let db = Db::open_in_memory().unwrap();
        let app = db.upsert_app(&safari(), 1000).unwrap();
        let seg = db
            .open_segment(app, None, SegState::Active, 1000)
            .unwrap();

        db.flush_segment(seg, 1030).unwrap();

        let (end_ts, dur): (i64, i64) = db
            .conn
            .query_row(
                "SELECT end_ts, duration_sec FROM segments WHERE id = ?1",
                [seg],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(end_ts, 1030);
        assert_eq!(dur, 30);

        let rows: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM usage_daily", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0, "flush must not touch rollup");
    }

    #[test]
    fn close_segment_updates_rollup_by_state() {
        let db = Db::open_in_memory().unwrap();
        let app = db.upsert_app(&safari(), 1000).unwrap();

        let s1 = db
            .open_segment(app, None, SegState::Active, 1000)
            .unwrap();
        db.close_segment(s1, 1060).unwrap();

        let s2 = db.open_segment(app, None, SegState::Idle, 1060).unwrap();
        db.close_segment(s2, 1090).unwrap();

        let s3 = db
            .open_segment(app, None, SegState::Active, 1090)
            .unwrap();
        db.close_segment(s3, 1100).unwrap();

        let (active, idle) = rollup(&db, &local_day(1000), app);
        assert_eq!(active, 70, "60 + 10 active");
        assert_eq!(idle, 30);
    }

    fn local_ts(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
        use chrono::{Local, TimeZone};
        Local
            .with_ymd_and_hms(y, mo, d, h, mi, s)
            .single()
            .unwrap()
            .timestamp()
    }

    #[test]
    fn close_segment_splits_at_midnight() {
        let db = Db::open_in_memory().unwrap();
        let app = db.upsert_app(&safari(), 1000).unwrap();

        let start = local_ts(2026, 6, 8, 23, 59, 0);
        let end = local_ts(2026, 6, 9, 0, 1, 0);
        let midnight = local_ts(2026, 6, 9, 0, 0, 0);

        let seg = db
            .open_segment(app, Some("Tarde"), SegState::Active, start)
            .unwrap();
        db.close_segment(seg, end).unwrap();

        let rows: Vec<(String, i64, i64, i64)> = {
            let mut stmt = db
                .conn
                .prepare(
                    "SELECT day, start_ts, end_ts, duration_sec
                     FROM segments ORDER BY start_ts",
                )
                .unwrap();
            stmt.query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
        };

        assert_eq!(rows.len(), 2, "segmento debe partirse en dos");
        assert_eq!(rows[0], ("2026-06-08".into(), start, midnight, 60));
        assert_eq!(rows[1], ("2026-06-09".into(), midnight, end, 60));

        assert_eq!(rollup(&db, "2026-06-08", app), (60, 0));
        assert_eq!(rollup(&db, "2026-06-09", app), (60, 0));
    }

    #[test]
    fn config_returns_spec_defaults_when_missing() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.config_i64("idle_threshold_sec").unwrap(), 60);
        assert_eq!(db.config_i64("poll_interval_ms").unwrap(), 1500);
        assert_eq!(db.config_i64("flush_interval_sec").unwrap(), 30);
        assert_eq!(db.config_i64("raw_retention_days").unwrap(), 180);
        assert_eq!(db.config_i64("top_apps_count").unwrap(), 10);
        assert!(db.config_bool("count_idle_as_usage").unwrap());
        assert!(db.config_bool("track_window_titles").unwrap());
        assert!(db.config_bool("autostart_enabled").unwrap());
        assert!(!db.config_bool("tracking_paused").unwrap());
        assert_eq!(db.config_str("language").unwrap(), "es");
    }

    #[test]
    fn config_set_then_get_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        db.set_config("idle_threshold_sec", "120").unwrap();
        assert_eq!(db.config_i64("idle_threshold_sec").unwrap(), 120);

        db.set_config("language", "en").unwrap();
        db.set_config("language", "es").unwrap();
        assert_eq!(db.config_str("language").unwrap(), "es");
    }

    #[test]
    fn open_segment_sets_marker_and_close_clears_it() {
        let db = Db::open_in_memory().unwrap();
        let app = db.upsert_app(&safari(), 1000).unwrap();

        let seg = db
            .open_segment(app, None, SegState::Active, 1000)
            .unwrap();
        assert_eq!(db.config_str("open_segment_id").unwrap(), seg.to_string());

        db.close_segment(seg, 1060).unwrap();
        assert_eq!(db.config_str("open_segment_id").unwrap(), "");
    }

    #[test]
    fn recover_closes_dangling_segment_at_last_flushed_end() {
        let db = Db::open_in_memory().unwrap();
        let app = db.upsert_app(&safari(), 1000).unwrap();

        // Simula cierre sucio: segmento abierto, flusheado, nunca cerrado.
        let seg = db
            .open_segment(app, None, SegState::Active, 1000)
            .unwrap();
        db.flush_segment(seg, 1030).unwrap();

        let recovered = db.recover().unwrap();
        assert_eq!(recovered, Some(seg));

        // Cerrado en el último end_ts conocido, sin rellenar el hueco (§13).
        let (end_ts, dur): (i64, i64) = db
            .conn
            .query_row(
                "SELECT end_ts, duration_sec FROM segments WHERE id = ?1",
                [seg],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(end_ts, 1030);
        assert_eq!(dur, 30);
        assert_eq!(rollup(&db, &local_day(1000), app), (30, 0));
        assert_eq!(db.config_str("open_segment_id").unwrap(), "");
    }

    #[test]
    fn recover_with_clean_state_does_nothing() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.recover().unwrap(), None);
    }

    fn vscode() -> AppInfo<'static> {
        AppInfo {
            identity: "C:\\Program Files\\VS Code\\Code.exe",
            display_name: "Code.exe",
            process_name: Some("Code.exe"),
            exe_path: Some("C:\\Program Files\\VS Code\\Code.exe"),
            bundle_id: None,
        }
    }

    #[test]
    fn rename_app_changes_display_name() {
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_app(&vscode(), 1000).unwrap();

        db.rename_app(id, "VS Code").unwrap();

        let name: String = db
            .conn
            .query_row("SELECT display_name FROM apps WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "VS Code");
    }

    #[test]
    fn merge_apps_moves_history_and_sums_rollups() {
        let db = Db::open_in_memory().unwrap();
        let a = db.upsert_app(&vscode(), 1000).unwrap();
        let b = db.upsert_app(&safari(), 1000).unwrap();
        let day = local_day(1000);

        let s1 = db.open_segment(a, None, SegState::Active, 1000).unwrap();
        db.close_segment(s1, 1060).unwrap();
        let s2 = db.open_segment(b, None, SegState::Active, 1060).unwrap();
        db.close_segment(s2, 1100).unwrap();

        db.merge_apps(a, b).unwrap();

        // Historia reasignada: todos los segmentos apuntan a b.
        let orphans: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM segments WHERE app_id = ?1",
                [a],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphans, 0);

        // Rollup sumado y fila origen eliminada.
        assert_eq!(rollup(&db, &day, b), (100, 0));
        let a_rows: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM usage_daily WHERE app_id = ?1",
                [a],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(a_rows, 0);

        // La app origen desaparece.
        let apps: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM apps WHERE id = ?1", [a], |r| r.get(0))
            .unwrap();
        assert_eq!(apps, 0);
    }
}

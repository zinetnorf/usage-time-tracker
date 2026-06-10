use crate::db::{AppInfo, Db, SegState};
use rusqlite::Result;

/// Ventana observada en un tick: identidad de la app + título.
pub struct ObservedWindow<'a> {
    pub app: AppInfo<'a>,
    pub title: Option<&'a str>,
}

/// Lo que el poller entrega al tracker en cada tick (§5.2).
/// `window: None` = no hay ventana en foreground (escritorio, etc.).
pub struct Observation<'a> {
    pub window: Option<ObservedWindow<'a>>,
    pub idle_seconds: i64,
    pub now_ts: i64,
}

struct OpenSeg {
    segment_id: i64,
    identity: String,
    state: SegState,
    last_flush_ts: i64,
}

/// Gap mínimo entre ticks para asumir suspensión no notificada (§17).
const SUSPEND_GAP_SEC: i64 = 30;

pub struct Tracker {
    db: Db,
    current: Option<OpenSeg>,
    blocked: bool,
    paused: bool,
    last_tick_ts: Option<i64>,
}

impl Tracker {
    pub fn new(db: Db) -> Result<Self> {
        db.recover()?;
        let paused = db.config_bool("tracking_paused")?;
        Ok(Tracker {
            db,
            current: None,
            blocked: false,
            paused,
            last_tick_ts: None,
        })
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Pausa manual desde el tray (§9): cierra el segmento y persiste el
    /// estado para sobrevivir reinicios (§17).
    pub fn pause(&mut self, now_ts: i64) -> Result<()> {
        self.paused = true;
        self.db.set_config("tracking_paused", "true")?;
        self.close_current(now_ts)
    }

    pub fn resume(&mut self) -> Result<()> {
        self.paused = false;
        self.db.set_config("tracking_paused", "false")
    }

    /// Salida limpia ("Salir" del tray): cierra el segmento en curso.
    pub fn shutdown(&mut self, now_ts: i64) -> Result<()> {
        self.close_current(now_ts)
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Cierra el segmento abierto, si lo hay.
    fn close_current(&mut self, now_ts: i64) -> Result<()> {
        if let Some(current) = self.current.take() {
            self.db.close_segment(current.segment_id, now_ts)?;
        }
        Ok(())
    }

    /// Bloqueo de sesión o suspensión (§5.3): cierra y pausa el conteo.
    pub fn on_lock(&mut self, now_ts: i64) -> Result<()> {
        self.blocked = true;
        self.close_current(now_ts)
    }

    pub fn on_unlock(&mut self) {
        self.blocked = false;
    }

    pub fn on_suspend(&mut self, now_ts: i64) -> Result<()> {
        self.on_lock(now_ts)
    }

    pub fn on_resume(&mut self) {
        self.on_unlock();
    }

    pub fn tick(&mut self, obs: &Observation) -> Result<()> {
        if self.blocked || self.paused {
            return Ok(());
        }

        // Fallback de suspensión: salto de reloj entre ticks → cerrar en
        // el último tick visto, sin contar el hueco.
        let last = self.last_tick_ts.replace(obs.now_ts);
        if let Some(last) = last {
            if obs.now_ts - last > SUSPEND_GAP_SEC {
                self.close_current(last)?;
            }
        }
        let Some(window) = &obs.window else {
            return self.close_current(obs.now_ts);
        };

        let threshold = self.db.config_i64("idle_threshold_sec")?;
        let state = if obs.idle_seconds >= threshold {
            SegState::Idle
        } else {
            SegState::Active
        };

        if let Some(current) = &mut self.current {
            if current.identity == window.app.identity && current.state == state {
                // Flush periódico anti-crash (§5.2.5), sin cerrar.
                let flush_interval = self.db.config_i64("flush_interval_sec")?;
                if obs.now_ts - current.last_flush_ts >= flush_interval {
                    self.db.flush_segment(current.segment_id, obs.now_ts)?;
                    current.last_flush_ts = obs.now_ts;
                }
                return Ok(());
            }
            // Cambió la app o el estado → cerrar y abrir (§5.2.4).
            self.db.close_segment(current.segment_id, obs.now_ts)?;
            self.current = None;
        }

        let app_id = self.db.upsert_app(&window.app, obs.now_ts)?;
        let segment_id = self.db.open_segment(app_id, window.title, state, obs.now_ts)?;
        self.current = Some(OpenSeg {
            segment_id,
            identity: window.app.identity.to_string(),
            state,
            last_flush_ts: obs.now_ts,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{AppInfo, Db};

    fn safari_obs(idle_seconds: i64, now_ts: i64) -> Observation<'static> {
        Observation {
            window: Some(ObservedWindow {
                app: AppInfo {
                    identity: "com.apple.Safari",
                    display_name: "Safari",
                    process_name: Some("Safari"),
                    exe_path: None,
                    bundle_id: Some("com.apple.Safari"),
                },
                title: Some("Inicio"),
            }),
            idle_seconds,
            now_ts,
        }
    }

    fn segments(t: &Tracker) -> Vec<(i64, String, i64, i64)> {
        let mut stmt = t
            .db()
            .conn
            .prepare("SELECT app_id, state, start_ts, end_ts FROM segments ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    #[test]
    fn first_tick_opens_active_segment() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();

        let segs = segments(&t);
        assert_eq!(segs.len(), 1);
        let (_, state, start_ts, _) = &segs[0];
        assert_eq!(state, "active");
        assert_eq!(*start_ts, 1000);
    }

    #[test]
    fn tick_same_app_and_state_does_not_open_new_segment() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(&safari_obs(1, 1002)).unwrap();
        t.tick(&safari_obs(0, 1003)).unwrap();

        assert_eq!(segments(&t).len(), 1);
    }

    fn vscode_obs(idle_seconds: i64, now_ts: i64) -> Observation<'static> {
        Observation {
            window: Some(ObservedWindow {
                app: AppInfo {
                    identity: "com.microsoft.VSCode",
                    display_name: "Code",
                    process_name: Some("Electron"),
                    exe_path: None,
                    bundle_id: Some("com.microsoft.VSCode"),
                },
                title: Some("main.rs"),
            }),
            idle_seconds,
            now_ts,
        }
    }

    #[test]
    fn app_change_closes_previous_and_opens_new() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(&vscode_obs(0, 1010)).unwrap();

        let segs = segments(&t);
        assert_eq!(segs.len(), 2);
        let (_, _, start0, end0) = &segs[0];
        assert_eq!((*start0, *end0), (1000, 1010), "anterior cerrado al tick del cambio");
        let (_, _, start1, _) = &segs[1];
        assert_eq!(*start1, 1010);

        // Rollup del segmento cerrado ya acumulado.
        let active: i64 = t
            .db()
            .conn
            .query_row("SELECT SUM(active_sec) FROM usage_daily", [], |r| r.get(0))
            .unwrap();
        assert_eq!(active, 10);
    }

    #[test]
    fn idle_threshold_switches_state_and_back() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        // Ticks con cadencia real (< gap de suspensión); idle_seconds es
        // del sistema y crece aunque los ticks sigan llegando.
        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(&safari_obs(28, 1028)).unwrap();
        t.tick(&safari_obs(56, 1056)).unwrap(); // aún < 60 → active
        t.tick(&safari_obs(61, 1061)).unwrap(); // >= 60 → idle
        t.tick(&safari_obs(0, 1063)).unwrap(); // input → active

        let segs = segments(&t);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].1, "active");
        assert_eq!(segs[1].1, "idle");
        assert_eq!(segs[2].1, "active");
        assert_eq!(segs[0].3, 1061, "active cerrado al pasar a idle");
        assert_eq!(segs[1].3, 1063, "idle cerrado al volver input");
    }

    fn open_end_ts(t: &Tracker) -> i64 {
        t.db()
            .conn
            .query_row("SELECT end_ts FROM segments ORDER BY id DESC LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap()
    }

    #[test]
    fn periodic_flush_persists_progress_without_closing() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(&safari_obs(0, 1010)).unwrap();
        assert_eq!(open_end_ts(&t), 1000, "antes del intervalo no flushea");

        t.tick(&safari_obs(0, 1031)).unwrap(); // >= 30 s desde apertura
        assert_eq!(open_end_ts(&t), 1031, "flush persiste avance");
        assert_eq!(segments(&t).len(), 1, "flush no cierra");

        // Sin rollup: el segmento sigue abierto.
        let rows: i64 = t
            .db()
            .conn
            .query_row("SELECT COUNT(*) FROM usage_daily", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn lock_closes_segment_and_blocks_until_unlock() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.on_lock(1050).unwrap();

        let segs = segments(&t);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].3, 1050, "cerrado al bloquear");

        // Ticks durante el bloqueo no cuentan.
        t.tick(&safari_obs(0, 1060)).unwrap();
        assert_eq!(segments(&t).len(), 1);

        // Al desbloquear, el siguiente tick abre nuevo; el hueco no se cuenta.
        t.on_unlock();
        t.tick(&safari_obs(0, 1100)).unwrap();
        let segs = segments(&t);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[1].2, 1100);
    }

    #[test]
    fn suspend_resume_behaves_like_lock() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.on_suspend(1030).unwrap();
        t.tick(&safari_obs(0, 1040)).unwrap();
        assert_eq!(segments(&t).len(), 1);
        assert_eq!(segments(&t)[0].3, 1030);

        t.on_resume();
        t.tick(&safari_obs(0, 1200)).unwrap();
        assert_eq!(segments(&t).len(), 2);
        assert_eq!(segments(&t)[1].2, 1200);
    }

    #[test]
    fn no_foreground_window_closes_current() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(
            &Observation {
                window: None,
                idle_seconds: 0,
                now_ts: 1020,
            },
        )
        .unwrap();

        let segs = segments(&t);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].3, 1020, "cerrado al perder foreground");
        assert_eq!(
            t.db().config_str("open_segment_id").unwrap(),
            "",
            "sin segmento abierto"
        );
    }

    #[test]
    fn clock_gap_closes_at_last_seen_tick() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(&safari_obs(0, 1002)).unwrap();
        // Gap pequeño (< umbral): el segmento continúa.
        t.tick(&safari_obs(0, 1010)).unwrap();
        assert_eq!(segments(&t).len(), 1);

        // Salto grande de reloj = suspensión perdida: cerrar en el último
        // tick visto, NO contar el hueco (decisión §17).
        t.tick(&safari_obs(0, 2000)).unwrap();
        let segs = segments(&t);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].3, 1010, "cerrado en el último tick visto");
        assert_eq!(segs[1].2, 2000, "nuevo segmento arranca tras el hueco");
    }

    #[test]
    fn pause_closes_segment_persists_and_ignores_ticks() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.pause(1020).unwrap();

        assert_eq!(segments(&t)[0].3, 1020, "cerrado al pausar");
        assert!(t.db().config_bool("tracking_paused").unwrap());

        t.tick(&safari_obs(0, 1030)).unwrap();
        assert_eq!(segments(&t).len(), 1, "pausado: ticks no cuentan");

        t.resume().unwrap();
        assert!(!t.db().config_bool("tracking_paused").unwrap());
        t.tick(&safari_obs(0, 1040)).unwrap();
        assert_eq!(segments(&t).len(), 2);
        assert_eq!(segments(&t)[1].2, 1040);
    }

    #[test]
    fn shutdown_closes_open_segment_at_now() {
        let db = Db::open_in_memory().unwrap();
        let mut t = Tracker::new(db).unwrap();

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.shutdown(1015).unwrap();

        assert_eq!(segments(&t)[0].3, 1015);
        assert_eq!(t.db().config_str("open_segment_id").unwrap(), "");
    }

    #[test]
    fn paused_state_survives_restart() {
        let db = Db::open_in_memory().unwrap();
        db.set_config("tracking_paused", "true").unwrap();

        // Simula reinicio: Tracker nuevo sobre una db ya pausada.
        let mut t = Tracker::new(db).unwrap();
        t.tick(&safari_obs(0, 1000)).unwrap();

        assert_eq!(segments(&t).len(), 0, "arranca pausado");
    }
}

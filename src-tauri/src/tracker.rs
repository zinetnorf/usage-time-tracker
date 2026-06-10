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
}

pub struct Tracker {
    db: Db,
    current: Option<OpenSeg>,
}

impl Tracker {
    pub fn new(db: Db) -> Result<Self> {
        db.recover()?;
        Ok(Tracker { db, current: None })
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn tick(&mut self, obs: &Observation) -> Result<()> {
        let Some(window) = &obs.window else {
            return Ok(());
        };

        let threshold = self.db.config_i64("idle_threshold_sec")?;
        let state = if obs.idle_seconds >= threshold {
            SegState::Idle
        } else {
            SegState::Active
        };

        if let Some(current) = &self.current {
            if current.identity == window.app.identity && current.state == state {
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

        t.tick(&safari_obs(0, 1000)).unwrap();
        t.tick(&safari_obs(70, 1070)).unwrap(); // >= 60 → idle
        t.tick(&safari_obs(0, 1080)).unwrap(); // input → active

        let segs = segments(&t);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].1, "active");
        assert_eq!(segs[1].1, "idle");
        assert_eq!(segs[2].1, "active");
        assert_eq!(segs[0].3, 1070, "active cerrado al pasar a idle");
        assert_eq!(segs[1].3, 1080, "idle cerrado al volver input");
    }
}

use crate::db::{local_day, Db};
use crate::platform::{self, Poller};
use crate::tracker::Tracker;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const PURGE_INTERVAL_SEC: i64 = 6 * 3600;

/// Control del thread de tracking: parada limpia desde "Salir".
pub struct LoopHandle {
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl LoopHandle {
    /// Detiene el loop y espera el cierre limpio del segmento en curso.
    /// El loop duerme en tramos de 100 ms, así que retorna enseguida.
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = self.join.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }
}

/// Lanza el tracker loop (§5.2) en su propio thread con su PROPIA
/// conexión SQLite (WAL: convive con la conexión de los comandos).
pub fn spawn(db_path: PathBuf, on_today: impl Fn(String) + Send + 'static) -> LoopHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);

    let handle = std::thread::spawn(move || {
        let db = match Db::open(&db_path) {
            Ok(db) => db,
            Err(e) => {
                eprintln!("tracker: no se pudo abrir la DB: {e}");
                return;
            }
        };
        let mut tracker = match Tracker::new(db) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("tracker: init falló: {e}");
                return;
            }
        };
        let mut poller = Poller::default();
        let mut last_purge = 0i64;
        let mut last_minute = -1i64;

        loop {
            let now = chrono::Local::now().timestamp();
            if let Err(e) = iteration(&mut tracker, &mut poller, now, &mut last_purge) {
                eprintln!("tracker: error en tick: {e}");
            }

            // Resumen rápido del tray, una vez por minuto.
            let minute = now / 60;
            if minute != last_minute {
                last_minute = minute;
                if let Ok(label) = today_label(&tracker) {
                    on_today(label);
                }
            }

            // Dormir en tramos cortos para responder rápido al stop.
            let poll_ms = tracker
                .db()
                .config_i64("poll_interval_ms")
                .unwrap_or(1500)
                .clamp(250, 60_000) as u64;
            let mut slept = 0u64;
            while slept < poll_ms {
                if stop_flag.load(Ordering::Relaxed) {
                    let now = chrono::Local::now().timestamp();
                    if let Err(e) = tracker.shutdown(now) {
                        eprintln!("tracker: cierre falló: {e}");
                    }
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
                slept += 100;
            }
        }
    });

    LoopHandle {
        stop,
        join: Mutex::new(Some(handle)),
    }
}

fn iteration(
    tracker: &mut Tracker,
    poller: &mut Poller,
    now: i64,
    last_purge: &mut i64,
) -> rusqlite::Result<()> {
    // Pausa manual: la UI/tray escriben config; el loop reacciona ≤1 tick.
    let want_pause = tracker.db().config_bool("tracking_paused")?;
    if want_pause && !tracker.is_paused() {
        tracker.pause(now)?;
    } else if !want_pause && tracker.is_paused() {
        tracker.resume()?;
    }

    // Bloqueo de sesión por sondeo (§5.3).
    if platform::is_session_locked() {
        tracker.on_lock(now)?;
    } else {
        tracker.on_unlock();
        let track_titles = tracker.db().config_bool("track_window_titles")?;
        let polled = poller.poll(now, track_titles);
        tracker.tick(&polled.as_observation())?;
    }

    if now - *last_purge > PURGE_INTERVAL_SEC {
        tracker.db().purge_old_segments(now)?;
        *last_purge = now;
    }
    Ok(())
}

/// "Hoy: 2h 15m" para el item del tray, respetando count_idle_as_usage.
fn today_label(tracker: &Tracker) -> rusqlite::Result<String> {
    let db = tracker.db();
    let today = local_day(chrono::Local::now().timestamp());
    let count_idle = db.config_bool("count_idle_as_usage")?;
    let total: i64 = db
        .day_summary(&today)?
        .iter()
        .map(|r| r.active_sec + if count_idle { r.idle_sec } else { 0 })
        .sum();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    Ok(if hours > 0 {
        format!("Hoy: {hours}h {minutes:02}m")
    } else {
        format!("Hoy: {minutes}m")
    })
}

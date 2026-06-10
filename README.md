# Usage Tracker

Desktop application that measures how much time you spend in each application. It runs in the background from the system tray, detects the foreground window, and distinguishes between **active time** (keyboard/mouse input) and **idle time**. Cross-platform: Windows 11 and macOS.

## 🔒 100% local, zero network

**This app's differentiator: your data never leaves your device.**

- No telemetry, no accounts, no cloud, no network calls of any kind.
- The database (SQLite) lives on your machine, and reports are generated and viewed only there.
- The code is open source: you can verify it.

| Platform | Data location |
|---|---|
| macOS | `~/Library/Application Support/com.davisg.usage-tracker/usage.db` |
| Windows | `%APPDATA%\com.davisg.usage-tracker\usage.db` |

## Features

- **Automatic tracking** of the foreground app, sampling every 1.5 s (configurable).
- **Active vs. idle:** after N seconds without input (60 by default) time counts as idle; you decide whether it adds up in reports.
- **Smart pause:** when the session locks or the machine sleeps, counting stops; the gap is never backfilled.
- **Dashboard** with 4 views:
  - **Today:** daily total + per-app chart + active/idle list.
  - **History:** 7/14/30-day trend + per-app breakdown.
  - **Apps:** rename, merge entries (e.g. `Code.exe` → "VS Code"), and exclude apps from tracking (blacklist).
  - **Settings:** idle threshold, retention, autostart, language…
- **PDF export:** the current view or a full report (excluded apps never appear in reports).
- **System tray:** quick daily summary, pause/resume, open dashboard. Closing the window does NOT quit the app.
- **Resilient:** if the app dies abruptly (crash, power loss), on relaunch it recovers the count without inflating or losing data.
- **Languages:** Spanish (default) and English.
- **Retention:** raw detail is purged after 180 days (configurable); daily aggregates are kept forever.

## Installation

Download the installer from [Releases](https://github.com/zinetnorf/usage-time-tracker/releases):

| Platform | File |
|---|---|
| macOS (Intel and Apple Silicon) | `Usage Tracker_x.y.z_universal.dmg` |
| Windows 11 | `Usage Tracker_x.y.z_x64-setup.exe` or `.msi` |

### macOS: first launch

> **⚠️ Note: this app is NOT signed with an Apple certificate.** macOS will block it by default, and you must allow it manually — this is expected for unsigned open-source apps and only needs to be done once.

1. Open the `.dmg` and drag **Usage Tracker** into Applications.
2. First launch: **right-click → Open** (instead of double-clicking), then confirm.
3. If macOS claims the app is "damaged", run in Terminal:
   ```bash
   xattr -cr "/Applications/Usage Tracker.app"
   ```
4. **Accessibility permission:** macOS requires it to read the active window title. The app's onboarding guides you to grant it (System Settings → Privacy & Security → Accessibility), and you must **relaunch the app** afterwards. Without the permission the app still works — just without window titles.

### Windows: first launch

SmartScreen will show "unknown publisher" (unsigned installer): **More info → Run anyway**. No special permissions required.

## Architecture

```
┌──────────────────────────────────────────────┐
│ Rust process (always alive)                  │
│  Tracker loop (1.5s poll) ──▶ SQLite (WAL)   │
│  - active window (x-win)         ▲           │
│  - system idle (user-idle)       │ invoke    │
│  - state machine                 │           │
│  Tray + power/lock detection     │           │
└──────────────────────────────────┼───────────┘
                                   │
                  React webview (only when opened)
                  Today · History · Apps · Settings
```

- **Core:** Tauri v2 + Rust. The tracking engine runs even with the window closed; the webview only loads when the dashboard opens.
- **UI:** React + TypeScript + Tailwind CSS + Recharts.
- **Data:** SQLite via `rusqlite` (WAL mode). Raw segments + daily per-app rollup. Versioned migrations.
- **Tracking:** every app or state change (active/idle) closes one segment and opens another; a segment crossing midnight is split per day. Periodic anti-crash flush of the in-progress segment.

## Configuration

All keys are editable from the **Settings** view:

| Key | Default | Description |
|---|---|---|
| `idle_threshold_sec` | 60 | Seconds without input before idle |
| `count_idle_as_usage` | yes | Whether idle time adds up in reports |
| `track_window_titles` | yes | Store the active window title |
| `poll_interval_ms` | 1500 | Sampling cadence |
| `raw_retention_days` | 180 | Days of raw detail before purging |
| `autostart_enabled` | yes | Launch at login |
| `language` | es | UI language (`es` / `en`) |

> **Known limitation:** without input it is impossible to tell "user away" from "user reading/watching content": both count as idle. That is why whether idle time counts is configurable.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for prerequisites, commands, and the release process.

## Roadmap

- [x] MVP: tracking + dashboard + tray + onboarding (macOS/Windows)
- [x] App blacklist and PDF export
- [ ] Per-window-title / per-website breakdown
- [ ] Categories and goals (productive vs. distraction)
- [ ] Installer signing and notarization + auto-update
- [ ] Linux
